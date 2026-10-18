"""Fake Claude Code CLI speaking the --print stream-json control protocol.

Frame shapes follow the transcripts recorded from Claude Code 2.1.296. It proves the
driver's framing, turn, permission, background and stop handling only.

Usage: fake-claude.py <mode> <root> [claude flags...]
Whole uppercase words in the prompt pick the behaviour of each turn (EDIT, ASK,
BACKGROUND [GATED] [HOLD], SLOW, STEER, TODO, NOLOGIN); anything else answers with text.
Observations go to <root>/*.json. Every turn writes <root>/writes.json ({path: text}) into
the working directory, and the turn Claude continues with by itself writes
<root>/late-writes.json; HOLD keeps the background task running until <root>/release exists.
"""

import json
import os
import re
import sys
import time
import uuid

MODE = sys.argv[1] if len(sys.argv) > 1 else "turns"
ROOT = sys.argv[2] if len(sys.argv) > 2 else "."
ARGS = sys.argv[3:]

if "--version" in ARGS:
    print("2.1.0 (Claude Code)" if MODE == "old" else "2.1.296 (Claude Code)")
    sys.exit(0)


def record(name, value):
    with open(os.path.join(ROOT, name), "w", encoding="utf-8") as f:
        json.dump(value, f)


def flag(name):
    for i, arg in enumerate(ARGS):
        if arg == name and i + 1 < len(ARGS):
            return ARGS[i + 1]
        if arg.startswith(name + "="):
            return arg.split("=", 1)[1]
    return None


SESSION = flag("--session-id") or flag("--resume") or str(uuid.uuid4())
if MODE == "wrong-session":
    SESSION = str(uuid.uuid4())
launch = {"args": ARGS}
for name in ("--mcp-config", "--settings"):
    path = flag(name)
    if path:
        with open(path, encoding="utf-8") as f:
            launch[name] = json.load(f)
record("launch.json", launch)


def send(frame):
    sys.stdout.write(json.dumps(frame) + "\n")
    sys.stdout.flush()


def system(subtype, **fields):
    send(dict(type="system", subtype=subtype, session_id=SESSION, uuid=str(uuid.uuid4()), **fields))


def delta(kind, text, parent=None):
    key = "text" if kind == "text_delta" else "thinking"
    send({"type": "stream_event", "event": {"type": "content_block_delta", "index": 0,
          "delta": {"type": kind, key: text}}, "session_id": SESSION, "parent_tool_use_id": parent})


def tool_use(tool_id, name, tool_input):
    send({"type": "assistant", "message": {"role": "assistant", "content": [
        {"type": "tool_use", "id": tool_id, "name": name, "input": tool_input}]},
        "parent_tool_use_id": None, "session_id": SESSION})


def tool_result(tool_id, content, is_error=False):
    send({"type": "user", "message": {"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": tool_id, "content": content, "is_error": is_error}]},
        "parent_tool_use_id": None, "session_id": SESSION})


def result(subtype="success", stop_reason="end_turn", is_error=False, text=""):
    send({"type": "result", "subtype": subtype, "stop_reason": stop_reason, "is_error": is_error,
          "result": text, "session_id": SESSION, "uuid": str(uuid.uuid4())})


queued = []
interrupted = [False]
log = []


def read():
    """Next stdin frame; answers to our requests are echoed like the real CLI does."""
    if queued:
        return queued.pop(0)
    line = sys.stdin.readline()
    if not line:
        record("log.json", log)
        sys.exit(0)
    frame = json.loads(line)
    log.append(frame)
    record("log.json", log)
    if frame.get("type") == "control_response":
        send(frame)
    if frame.get("type") == "control_request" and frame["request"].get("subtype") == "interrupt":
        interrupted[0] = True
        send({"type": "control_response", "response": {"subtype": "success",
              "request_id": frame["request_id"], "response": {"still_queued": [], "cancelled": []}}})
    return frame


def ask(request_id, tool_name, tool_input, tool_id):
    send({"type": "control_request", "request_id": request_id, "request": {
        "subtype": "can_use_tool", "tool_name": tool_name, "display_name": tool_name,
        "input": tool_input, "description": "fixture", "tool_use_id": tool_id}})
    while True:
        frame = read()
        if frame.get("type") == "control_response" and frame["response"].get("request_id") == request_id:
            return frame["response"]["response"]
        if interrupted[0]:
            return None
        if frame.get("type") == "user":
            queued.append(frame)


def write_files(name):
    """Writes the files listed in <root>/<name> into the working directory."""
    path = os.path.join(ROOT, name)
    if not os.path.exists(path):
        return
    with open(path, encoding="utf-8") as f:
        files = json.load(f)
    for relative, contents in files.items():
        target = os.path.join(os.getcwd(), relative)
        os.makedirs(os.path.dirname(target), exist_ok=True)
        with open(target, "w", encoding="utf-8", newline="\n") as f:
            f.write(contents)


def turn(frame):
    content = frame["message"]["content"]
    prose = content if isinstance(content, str) else " ".join(
        b.get("text", "") for b in content if b.get("type") == "text")
    text = set(re.findall(r"\b[A-Z]+\b", prose))
    if isinstance(content, list):
        record("images.json", [b["source"]["media_type"] for b in content if b.get("type") == "image"])
    system("init", cwd=os.getcwd(), tools=["Bash", "Edit", "AskUserQuestion"],
           slash_commands=["compact", "/review", "init"])
    send({"type": "user", "message": frame["message"], "session_id": SESSION,
          "parent_tool_use_id": None, "uuid": frame.get("uuid"), "isReplay": True})
    if "NOLOGIN" in text:
        result(is_error=True, stop_reason="stop_sequence", text="Not logged in · Please run /login")
        return
    delta("thinking_delta", "thinking about it")
    delta("text_delta", "on it")
    delta("text_delta", "subagent text", parent="toolu_parent")
    write_files("writes.json")
    if "EDIT" in text:
        tool_use("toolu_edit", "Edit", {"file_path": "hello.txt", "description": "edit hello"})
        answer = ask("perm-edit", "Edit", {"file_path": "hello.txt"}, "toolu_edit")
        allowed = answer is not None and answer.get("behavior") == "allow"
        if allowed:
            with open(os.path.join(ROOT, "hello.txt"), "w", encoding="utf-8", newline="\n") as f:
                f.write("allowed\n")
        record("permission.json", answer)
        tool_result("toolu_edit", "done" if allowed else "denied", is_error=not allowed)
    if "ASK" in text:
        questions = [
            {"header": "Color", "question": "Which color?", "options": [{"label": "Red", "description": "warm"}, "Blue"]},
            {"question": "Which sizes?", "options": ["S", "M"], "multiSelect": True},
        ]
        tool_use("toolu_ask", "AskUserQuestion", {"questions": questions})
        record("question.json", ask("perm-ask", "AskUserQuestion", {"questions": questions}, "toolu_ask"))
        tool_result("toolu_ask", "answered")
    if "TODO" in text:
        tool_use("toolu_todo", "TodoWrite", {"todos": [
            {"content": "First", "status": "completed"}, {"content": "Second", "status": "in_progress"}]})
        tool_result("toolu_todo", "ok")
    if "STEER" in text:
        while True:
            steer = read()
            if steer.get("type") == "user":
                break
        # Taken up after the turn already reported its end, as a queued message is.
        result()
        system("init", cwd=os.getcwd(), tools=[])
        send({"type": "user", "message": steer["message"], "session_id": SESSION,
              "parent_tool_use_id": None, "uuid": steer.get("uuid"), "isReplay": True})
        record("steer.json", steer)
        delta("text_delta", "steered")
        result()
        return
    if "SLOW" in text:
        while not interrupted[0]:
            frame = read()
            if frame.get("type") == "user":
                queued.append(frame)
        interrupted[0] = False
        result(subtype="error_during_execution", stop_reason=None, is_error=True)
        return
    if "BACKGROUND" in text:
        tool_use("toolu_bg", "Bash", {"command": "sleep", "run_in_background": True})
        system("background_tasks_changed", tasks=[{"task_id": "bg1", "task_type": "local_bash"}])
        tool_result("toolu_bg", "Command running in background with ID: bg1.")
        delta("text_delta", "started")
        result()
        time.sleep(0.5)
        while "HOLD" in text and not os.path.exists(os.path.join(ROOT, "release")):
            time.sleep(0.05)
        system("task_notification", task_id="bg1", status="completed", summary="done")
        system("background_tasks_changed", tasks=[])
        if MODE == "no-continue":
            return
        system("init", cwd=os.getcwd(), tools=[])
        delta("text_delta", "background finished")
        write_files("late-writes.json")
        if "GATED" in text:
            tool_use("toolu_late", "Edit", {"file_path": "hello.txt"})
            record("autonomous-permission.json", ask("perm-late", "Edit", {"file_path": "hello.txt"}, "toolu_late"))
            tool_result("toolu_late", "done")
        result()
        return
    result()


while True:
    frame = read()
    kind = frame.get("type")
    if kind == "control_request" and frame["request"].get("subtype") == "initialize":
        if MODE == "no-init":
            continue
        send({"type": "control_response", "response": {"subtype": "success", "request_id": frame["request_id"],
              "response": {"claude_code_version": "2.1.296", "account": "fixture", "commands": []}}})
    elif kind == "user":
        turn(frame)

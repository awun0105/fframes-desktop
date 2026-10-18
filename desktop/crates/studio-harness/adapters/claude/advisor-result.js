            case "advisor_tool_result": {
                // Studio patch for claude-agent-acp 0.89.1: the API already
                // returned this server tool's result. Close that exact call;
                // never infer completion from its id or the turn's final text.
                const id = chunk.tool_use_id;
                if (typeof id !== "string") break;
                const emitted = options?.emittedToolCalls?.delete(id) === true;
                completeHookCallback(id);
                finishedToolCallId = id;
                const toolUse = toolUseCache[id];
                delete toolUseCache[id];
                if (emitted) {
                    output.push({
                        sessionId,
                        update: {
                            sessionUpdate: "tool_call_update",
                            toolCallId: id,
                            status: chunk.content?.type === "advisor_tool_result_error"
                                ? "failed" : "completed",
                            rawOutput: chunk.content,
                            ...(toolUse ? { _meta: { claudeCode: { toolName: toolUse.name } } } : {}),
                        },
                    });
                }
                break;
            }

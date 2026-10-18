import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';

const {toAcpNotifications} = await import(pathToFileURL(process.argv[2]).href);
for (const [content, status] of [
  [{type: 'advisor_tool_result', content: 'done'}, 'completed'],
  [{type: 'advisor_tool_result_error', error_code: 'execution_time_exceeded'}, 'failed'],
]) {
  const id = 'srvtoolu_advisor';
  const cache = {[id]: {id, name: 'advisor', input: {}}};
  const emittedToolCalls = new Set([id, 'toolu_edit_still_running']);
  const result = toAcpNotifications(
    [{type: 'advisor_tool_result', tool_use_id: id, content}],
    'assistant', 'test', cache, {}, console,
    {emittedToolCalls, registerHooks: false},
  );
  assert.equal(result.length, 1);
  assert.equal(result[0].update.toolCallId, id);
  assert.equal(result[0].update.status, status);
  assert.deepEqual(result[0].update.rawOutput, content);
  assert.equal(cache[id], undefined);
  assert.deepEqual([...emittedToolCalls], ['toolu_edit_still_running']);
  // A replay of the same result never manufactures a second tool completion.
  assert.deepEqual(toAcpNotifications(
    [{type: 'advisor_tool_result', tool_use_id: id, content}],
    'assistant', 'test', cache, {}, console, {emittedToolCalls, registerHooks: false},
  ), []);
}
console.log('Advisor success, error, duplicate and unrelated unfinished call: pass');

import { describe, expect, it } from 'vitest';
import { MAX_REQUEST, wsTransport, type Socket } from './ws';

/** A WebSocket with no network: what the page sends is kept, what the server says is pushed by the test. */
class FakeSocket implements Socket {
  binaryType: BinaryType = 'blob';
  sent: string[] = [];
  closed = false;
  onmessage: Socket['onmessage'] = null;
  onclose: Socket['onclose'] = null;
  send(data: string) {
    this.sent.push(data);
  }
  close() {
    this.closed = true;
    this.onclose?.call(this as unknown as WebSocket, new CloseEvent('close'));
  }
  frame(data: string | ArrayBuffer) {
    this.onmessage?.call(this as unknown as WebSocket, new MessageEvent('message', { data }));
  }
  text(value: unknown) {
    this.frame(JSON.stringify(value));
  }
  hello(protocol = 1) {
    this.text({ hello: { version: '2.2.3', protocol } });
  }
  /** A terminal's bytes, as the server frames them: the stream number on 4 bytes, big-endian, then the bytes. */
  bytes(stream: number, text: string) {
    const body = new TextEncoder().encode(text);
    const frame = new Uint8Array(4 + body.length);
    new DataView(frame.buffer).setUint32(0, stream);
    frame.set(body, 4);
    this.frame(frame.buffer);
  }
  requests(): { id: number; cmd: string; args: Record<string, unknown>; stream?: number }[] {
    return this.sent.map((s) => JSON.parse(s));
  }
}

/** The socket opens at the first call: the server's greeting comes after it. */
function connected() {
  const socket = new FakeSocket();
  const transport = wsTransport(() => socket);
  return { socket, transport };
}

const text = (b: ArrayBuffer) => new TextDecoder().decode(b);

describe('the WebSocket transport', () => {
  it('an_answer_resolves_its_own_call', async () => {
    const { socket, transport } = connected();
    const calls = ['a', 'b', 'c'].map((id) => transport.invoke('get_agent', { id }));
    socket.hello();
    expect(socket.binaryType).toBe('arraybuffer');
    const [a, b, c] = socket.requests();
    socket.text({ id: c.id, ok: 'C' });
    socket.text({ id: a.id, ok: 'A' });
    socket.text({ id: b.id, ok: null });
    expect(await Promise.all(calls)).toEqual(['A', null, 'C']);
    expect(a).toEqual({ id: a.id, cmd: 'get_agent', args: { id: 'a' } });
  });

  it('a_failed_command_rejects_with_the_value_the_window_gets', async () => {
    const { socket, transport } = connected();
    const plain = transport.invoke('create_project', {});
    const structured = transport.invoke('git_push', {});
    socket.hello();
    const [p, s] = socket.requests();
    const error = { kind: 'rejected', remote: 'origin', lines: ['! [rejected]'] };
    socket.text({ id: p.id, err: { kind: 'failed', message: 'the folder is gone' } });
    socket.text({ id: s.id, err: { kind: 'failed', message: error } });
    await expect(plain).rejects.toBe('the folder is gone');
    await expect(structured).rejects.toEqual(error);
    await structured.catch((e) => expect(e).not.toBeInstanceOf(Error));
  });

  it('the_other_kinds_reject_with_an_error_that_names_them', async () => {
    const { socket, transport } = connected();
    const kinds = ['unknown', 'desktop', 'args'];
    const calls = kinds.map((kind) => transport.invoke(`cmd_${kind}`));
    socket.hello();
    socket.requests().forEach((r, i) => socket.text({ id: r.id, err: { kind: kinds[i], message: 'why' } }));
    for (const [i, call] of calls.entries()) {
      const e = (await call.catch((e) => e)) as Error;
      expect(e).toBeInstanceOf(Error);
      expect(e.message).toContain(kinds[i]);
      expect(e.message).toContain(`cmd_${kinds[i]}`);
    }
  });

  it('the_calls_leave_in_the_order_they_were_made', () => {
    const { socket, transport } = connected();
    // Half before the greeting (kept until it is read), half after.
    for (let i = 0; i < 100; i++) void transport.invoke('term_write', { id: 't1', data: String(i) });
    socket.hello();
    for (let i = 100; i < 200; i++) void transport.invoke('term_write', { id: 't1', data: String(i) });
    const sent = socket.requests();
    expect(sent.map((r) => r.args.data)).toEqual(Array.from({ length: 200 }, (_, i) => String(i)));
    expect(new Set(sent.map((r) => r.id)).size).toBe(200);
  });

  it('an_event_reaches_the_flow_given_to_subscribe', async () => {
    const { socket, transport } = connected();
    const events: unknown[] = [];
    const channel = transport.channel<unknown>();
    channel.onmessage = (e) => events.push(e);
    const state = transport.invoke('subscribe', { channel });
    socket.hello();
    const [r] = socket.requests();
    expect(r.args).toEqual({});
    // The server registers the subscriber before it reads the state: an event can come before the answer.
    socket.text({ event: { type: 'agentUpdated', id: 'a1' } });
    socket.text({ id: r.id, ok: { projects: [] } });
    socket.text({ event: { type: 'agentRemoved', id: 'a1' } });
    expect(await state).toEqual({ projects: [] });
    expect(events).toEqual([
      { type: 'agentUpdated', id: 'a1' },
      { type: 'agentRemoved', id: 'a1' },
    ]);
  });

  it('a_terminals_bytes_reach_the_flow_of_their_number', async () => {
    const { socket, transport } = connected();
    const got: Record<string, ArrayBuffer[]> = { a: [], b: [] };
    const a = transport.channel<ArrayBuffer>();
    a.onmessage = (d) => got.a.push(d);
    const b = transport.channel<ArrayBuffer>();
    b.onmessage = (d) => got.b.push(d);
    void transport.invoke('term_spawn', { projectId: 'p1', cols: 80, rows: 24, output: a });
    void transport.invoke('run_start', { projectId: 'p1', cols: 80, rows: 24, output: b });
    socket.hello();
    const [ra, rb] = socket.requests();
    expect(ra.args).toEqual({ projectId: 'p1', cols: 80, rows: 24 });
    expect(ra.stream).not.toBe(rb.stream);
    socket.bytes(ra.stream!, 'a1');
    socket.bytes(rb.stream!, 'b1');
    socket.bytes(ra.stream!, 'a2');
    socket.bytes(rb.stream!, 'b2');
    expect(got.a.every((d) => d instanceof ArrayBuffer)).toBe(true);
    expect(got.a.map(text)).toEqual(['a1', 'a2']);
    expect(got.b.map(text)).toEqual(['b1', 'b2']);
  });

  it('the_greeting_is_read_before_any_call', async () => {
    const { socket, transport } = connected();
    const call = transport.invoke('mcp_status');
    expect(socket.sent).toEqual([]);
    socket.hello();
    const [r] = socket.requests();
    expect(r.cmd).toBe('mcp_status');
    socket.text({ id: r.id, ok: 'on' });
    expect(await call).toBe('on');
  });

  it('a_protocol_the_client_does_not_know_is_refused_with_a_message', async () => {
    const { socket, transport } = connected();
    const call = transport.invoke('mcp_status');
    socket.hello(2);
    const e = (await call.catch((e) => e)) as Error;
    expect(e).toBeInstanceOf(Error);
    expect(e.message).toContain('protocol 2');
    expect(socket.sent).toEqual([]);
    expect(socket.closed).toBe(true);
  });

  it('a_lost_connection_rejects_the_calls_in_flight', async () => {
    const { socket, transport } = connected();
    const lost: string[] = [];
    transport.onClose((reason) => lost.push(reason));
    const calls = [transport.invoke('mcp_status'), transport.invoke('list_agents')];
    socket.hello();
    socket.close();
    for (const call of calls) expect(await call.catch((e) => e)).toBeInstanceOf(Error);
    expect(lost).toHaveLength(1);
  });
  it('a_request_the_server_would_not_read_is_refused_here_and_the_connection_kept', async () => {
    const { socket, transport } = connected();
    const before = transport.invoke('mcp_status');
    socket.hello();
    // An image pasted, in base64: past the server's limit by its bytes, not by its letters (« é » is 2).
    const big = transport.invoke('paste_image', { id: 'a1', data: 'é'.repeat(MAX_REQUEST / 2) });
    const e = (await big.catch((e) => e)) as Error;
    expect(e).toBeInstanceOf(Error);
    expect(e.message).toMatch(/^paste_image: args: too large/);
    expect(socket.requests().map((r) => r.cmd)).toEqual(['mcp_status']);
    expect(socket.closed).toBe(false);
    const [r] = socket.requests();
    socket.text({ id: r.id, ok: 'on' });
    expect(await before).toBe('on');
    // Just under the limit, it goes.
    void transport.invoke('paste_image', { data: 'x'.repeat(MAX_REQUEST - 100) });
    expect(socket.requests().map((r) => r.cmd)).toEqual(['mcp_status', 'paste_image']);
  });

  it('a_terminals_flow_is_let_go_at_its_exit', async () => {
    const { socket, transport } = connected();
    const got: string[] = [];
    const events = transport.channel<unknown>();
    void transport.invoke('subscribe', { channel: events });
    const output = transport.channel<ArrayBuffer>();
    output.onmessage = (d) => got.push(text(d));
    const term = transport.invoke('term_spawn', { projectId: 'p1', cols: 80, rows: 24, output });
    socket.hello();
    const [, r] = socket.requests();
    socket.text({ id: r.id, ok: { id: 't1', projectId: 'p1', name: 'zsh', shell: 'zsh' } });
    await term;
    socket.bytes(r.stream!, 'before');
    // Another terminal's exit lets nothing go.
    socket.text({ event: { type: 'terminalExit', id: 't2', code: 0 } });
    socket.bytes(r.stream!, 'still');
    socket.text({ event: { type: 'terminalExit', id: 't1', code: 0 } });
    socket.bytes(r.stream!, 'after');
    expect(got).toEqual(['before', 'still']);
  });
});

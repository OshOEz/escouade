// `claude` started alone on a server without a browser, as its sign-in goes there: it tries $BROWSER (Escouade's opener,
// which finds nothing to open on a server) and goes on; the address to open shown in the terminal, as long as a real one
// (some 400 characters, over several rows), the code the sign-in page gives pasted back. The code is `e2e-code#<state>`, the state that of the address
// (anything else is refused, and asked again); then the sign-in is written where Claude Code keeps it, in
// $CLAUDE_CONFIG_DIR: .claude.json (who it is) first, then .credentials.json.
import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const dir = process.env.CLAUDE_CONFIG_DIR;
const state = crypto.randomBytes(16).toString('hex');
const challenge = crypto.randomBytes(32).toString('base64url');
const url =
  'https://claude.ai/oauth/authorize?code=true&client_id=9d1c250a-e61b-44d9-88ed-5944d1962f5e&response_type=code' +
  '&redirect_uri=https%3A%2F%2Fconsole.anthropic.com%2Foauth%2Fcode%2Fcallback' +
  '&scope=org%3Acreate_api_key+user%3Aprofile+user%3Ainference+user%3Asessions%3Aclaude_code' +
  `&code_challenge=${challenge}&code_challenge_method=S256&state=${state}`;
const ask = 'Paste code here if prompted > ';
const say = (text) => process.stdout.write(text);

if (process.stdin.isTTY) process.stdin.setRawMode(true);
// As Claude Code: the browser tried, whatever comes of it.
if (process.env.BROWSER)
  spawn(process.env.BROWSER, [url], { detached: true, stdio: 'ignore' })
    .on('error', () => {})
    .unref();
say(`Browser didn't open? Use the url below to sign in:\r\n\r\n${url}\r\n\r\n${ask}`);
let line = '';
// Signed in, it stays (as Claude Code goes on to its session) until Escouade closes it.
let done = false;
process.stdin.on('data', (data) => {
  for (const c of data.toString('utf8')) {
    if (c === '\x03') process.exit(130);
    if (done) continue;
    if (c !== '\r' && c !== '\n') {
      line += c;
      say(c);
      continue;
    }
    if (line !== `e2e-code#${state}`) {
      line = '';
      say(`\r\nOAuth error: invalid code\r\n${ask}`);
      continue;
    }
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(path.join(dir, '.claude.json'), JSON.stringify({ oauthAccount: { emailAddress: 'ada@exemple.test' } }));
    fs.writeFileSync(
      path.join(dir, '.credentials.json'),
      JSON.stringify({ claudeAiOauth: { accessToken: 'tok-e2e', refreshToken: 'ref-e2e', expiresAt: 4_102_444_800_000 } }),
    );
    say('\r\nLogin successful.\r\n');
    done = true;
  }
});

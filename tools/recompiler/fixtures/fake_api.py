import http.server, json
RUST = open('fixtures/sds_ll2str_mock.md').read()
class H(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        n = int(self.headers['Content-Length']); body = json.loads(self.rfile.read(n))
        assert self.headers['Authorization'] == 'Bearer testkey', 'auth'
        assert body['messages'][0]['role'] == 'system'
        out = {"choices": [{"message": {"role": "assistant", "content": RUST}}]}
        b = json.dumps(out).encode(); self.send_response(200)
        self.send_header('Content-Type','application/json'); self.send_header('Content-Length', str(len(b))); self.end_headers(); self.wfile.write(b)
    def log_message(self,*a): pass
http.server.HTTPServer(('127.0.0.1', 8765), H).serve_forever()

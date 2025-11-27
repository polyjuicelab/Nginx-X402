#!/usr/bin/env python3
"""Simple mock backend server for integration testing"""
import http.server
import socketserver
import json

class MockBackendHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        response = {"status": "ok", "message": "Backend response", "path": self.path}
        self.wfile.write(json.dumps(response).encode())
    
    def do_POST(self):
        self.do_GET()
    
    def log_message(self, format, *args):
        pass  # Suppress logging

if __name__ == "__main__":
    with socketserver.TCPServer(("", 9999), MockBackendHandler) as httpd:
        httpd.serve_forever()


#!/usr/bin/env python3
"""Simple mock backend server for integration testing"""
import http.server
import socketserver
import json

class MockBackendHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        # Add CORS headers for cross-origin requests
        self._add_cors_headers()
        self.end_headers()
        response = {"status": "ok", "message": "Backend response", "path": self.path}
        self.wfile.write(json.dumps(response).encode())
    
    def do_POST(self):
        self.do_GET()
    
    def do_OPTIONS(self):
        # Handle CORS preflight requests
        self.send_response(204)
        self._add_cors_headers()
        # Add CORS preflight headers
        requested_method = self.headers.get("Access-Control-Request-Method", "GET, POST, PUT, DELETE, OPTIONS")
        requested_headers = self.headers.get("Access-Control-Request-Headers", "content-type, authorization, x-payment")
        self.send_header("Access-Control-Allow-Methods", requested_method)
        self.send_header("Access-Control-Allow-Headers", requested_headers)
        self.send_header("Access-Control-Max-Age", "86400")
        self.end_headers()
    
    def do_HEAD(self):
        # Handle HEAD requests
        # HEAD should return the same headers as GET (except body)
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        # Calculate Content-Length based on what GET would return
        response = {"status": "ok", "message": "Backend response", "path": self.path}
        content_length = len(json.dumps(response).encode())
        self.send_header("Content-Length", str(content_length))
        self._add_cors_headers()
        self.end_headers()
    
    def do_TRACE(self):
        # Handle TRACE requests (often disabled for security)
        self.send_response(405)
        self.send_header("Content-Type", "text/plain")
        self._add_cors_headers()
        self.end_headers()
        self.wfile.write(b"Method Not Allowed")
    
    def _add_cors_headers(self):
        # Add CORS headers if Origin header is present
        origin = self.headers.get("Origin")
        if origin:
            self.send_header("Access-Control-Allow-Origin", origin)
            self.send_header("Access-Control-Allow-Credentials", "true")
    
    def log_message(self, format, *args):
        pass  # Suppress logging

if __name__ == "__main__":
    with socketserver.TCPServer(("", 9999), MockBackendHandler) as httpd:
        httpd.serve_forever()


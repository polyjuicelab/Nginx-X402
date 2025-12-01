#!/usr/bin/env python3
"""Flask-based mock backend server for integration testing"""
from flask import Flask, jsonify, request, Response
import os

app = Flask(__name__)

@app.route('/', defaults={'path': ''}, methods=['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS', 'PATCH'])
@app.route('/<path:path>', methods=['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS', 'PATCH'])
def handle_request(path):
    """Handle all requests with CORS and custom headers support"""
    
    # Prepare response data
    response_data = {
        "status": "ok",
        "message": "Backend response",
        "path": f"/{path}" if path else "/",
        "method": request.method
    }
    
    # Create response
    response = jsonify(response_data)
    
    # Add CORS headers
    response.headers['Access-Control-Allow-Origin'] = '*'
    response.headers['Access-Control-Allow-Methods'] = 'GET, POST, PUT, DELETE, OPTIONS, PATCH'
    response.headers['Access-Control-Allow-Headers'] = 'Content-Type, Authorization, X-PAYMENT, X-Custom-Header'
    response.headers['Access-Control-Expose-Headers'] = 'X-Custom-Response-Header, X-Another-Custom-Header'
    response.headers['Access-Control-Max-Age'] = '3600'
    
    # Add custom headers for testing passthrough
    response.headers['X-Custom-Response-Header'] = 'custom-value-123'
    response.headers['X-Another-Custom-Header'] = 'another-value-456'
    response.headers['X-Backend-Version'] = '1.0.0'
    response.headers['X-Request-ID'] = request.headers.get('X-Request-ID', 'not-provided')
    response.headers['X-BACKEND-TEST'] = 'backend-header-value'
    
    # Handle OPTIONS preflight requests
    if request.method == 'OPTIONS':
        return response, 200
    
    return response, 200

if __name__ == "__main__":
    port = int(os.environ.get('PORT', 9999))
    app.run(host='0.0.0.0', port=port, debug=False)

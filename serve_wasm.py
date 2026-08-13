import http.server
import os

class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".otf": "font/otf",
        ".ttf": "font/ttf",
        ".png": "image/png",
        ".jpg": "image/jpeg",
        ".ogg": "audio/ogg",
        ".opus": "audio/ogg",
        ".mp3": "audio/mpeg",
    }
    def log_message(self, *args):
        pass

os.chdir(".")
http.server.ThreadingHTTPServer(("0.0.0.0", 8080), Handler).serve_forever()

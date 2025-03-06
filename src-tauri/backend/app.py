from flask import Flask, jsonify, send_file, request
from flask_cors import CORS
import sys
import os
import time
import logging
import threading
import queue

from lib.db import *
from lib.kavita import *
from lib.kavita import get_appdir_path

# Setup logging system
log_buffer = []
MAX_LOG_ENTRIES = 1000

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='[%(levelname)s] %(asctime)s - %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)

# Create a custom handler to capture logs in the buffer
class BufferHandler(logging.Handler):
    def emit(self, record):
        log_message = self.format(record)
        log_buffer.append(log_message)
        if len(log_buffer) > MAX_LOG_ENTRIES:
            log_buffer.pop(0)
        # Also print to console
        print(log_message, file=sys.__stdout__)

# Add the buffer handler to the root logger
buffer_handler = BufferHandler()
buffer_handler.setFormatter(logging.Formatter('[%(levelname)s] %(asctime)s - %(message)s'))
logging.getLogger().addHandler(buffer_handler)

# Create module logger
logger = logging.getLogger(__name__)

# Log startup message
logger.info("Backend server starting")

# Default IP address (will be overridden by database settings if available)
DEFAULT_IP = "192.168.5.75:5002"

g_root = os.path.dirname(os.path.abspath(__file__))

username = "boddicheg"
password = "dyd6ZNU.aby*mqd6fwd"
api_key = "8df0fde8-8229-464c-ae0c-fd58a1a35b11"

# Add function to get cache size
def get_cache_size():
    try:
        cache_path = os.path.join(get_appdir_path("cache"))
        total_size = 0
        for dirpath, dirnames, filenames in os.walk(cache_path):
            for f in filenames:
                fp = os.path.join(dirpath, f)
                total_size += os.path.getsize(fp)
        return round(total_size / (1024 * 1024), 2)  # Return size in MB
    except Exception as e:
        logger.error(f"Error getting cache size: {e}")
        return 0

app = Flask(__name__)
try:
    # Initialize the database first to check for saved settings
    db = DBSession(DB_PATH)
    saved_ip = db.get_server_setting("server_ip")
    
    # Use saved IP if available, otherwise use default
    current_ip = saved_ip if saved_ip else DEFAULT_IP
    logger.info(f"Using IP address: {current_ip} (from {'database' if saved_ip else 'default'})")
    
    # Initialize KavitaAPI with the current IP
    kavita = KavitaAPI(current_ip, username, password, api_key)
    logger.info("KavitaAPI initialized")
except Exception as e:
    logger.error(f"Error initializing KavitaAPI: {e}")
    sys.exit(1)

CORS(app)

@app.route('/api/status')
def status():
    try:
        is_offline = kavita.get_offline_mode()
        ip = kavita.get_kavita_ip()
        logged = kavita.logged_as
        cache_size = get_cache_size()
        logger.info(f"Status request: offline={is_offline}, ip={ip}, logged_as={logged}, cache={cache_size}")
        return jsonify(status=not is_offline, ip=ip, logged_as=logged, cache=cache_size)
    except Exception as e:
        logger.error(f"Error in status route: {e}")
        return jsonify(status=False, ip=DEFAULT_IP, logged_as="", cache=0, error=str(e))

@app.route('/api/library')
def library():
    libraries = kavita.get_library()
    return jsonify(libraries)

@app.route('/api/series/<library>', methods=['GET'])
def series(library):
    s = kavita.get_series(str(library))
    # precaching serie covers
    for serie in s:
        kavita.get_series_cover(str(serie["id"]))
        serie["cached"] = kavita.is_series_cached(serie["id"])
    return jsonify(s)

@app.route('/api/series-cover/<id>', methods=['GET'])
def series_cover(id):
    cover = kavita.get_series_cover(str(id))
    return send_file(cover, as_attachment=True)

@app.route('/api/volumes/<series>', methods=['GET'])
def volumes(series):
    v = kavita.get_volumes(str(series))
    # precaching volume covers
    for volume in v:
        kavita.get_volume_cover(str(volume["volume_id"]))
        volume["cached"] = kavita.is_volume_cached(volume["volume_id"])
    return jsonify(v)

@app.route('/api/volumes-cover/<volume>', methods=['GET'])
def volume_cover(volume):
    cover = kavita.get_volume_cover(str(volume))
    return send_file(cover, as_attachment=True)

@app.route('/api/picture/<series>/<volume>/<chapter>/<page>', methods=['GET'])
def picture(series, volume, chapter, page):
    image = kavita.get_picture(chapter, page)
    kavita.save_progress({
        "series_id": series,
        "volume_id": volume,
        "chapter_id": chapter,
        "page": page
    })
    return send_file(image, as_attachment=True)

@app.route('/api/clear-cache', methods=['GET'])
def cache_clear():
    kavita.clear_manga_cache()
    return jsonify(status="success")

@app.route('/api/cache/serie/<id>', methods=['GET'])
def cache_serie(id):
    try:
        kavita.cache_serie({ "id": int(id), "title": ""}, None)
    except Exception as e:
        print(e)
    return jsonify(status="started")

@app.route('/api/update-lib', methods=['GET'])
def update_lib():
    kavita.update_server_library()
    return jsonify(status="success")

@app.route('/api/read-volume/<series_id>/<volume_id>', methods=['GET'])
def read_volume(series_id, volume_id):
    kavita.set_volume_as_read(series_id, volume_id)
    return jsonify(status="success")

@app.route('/api/unread-volume/<series_id>/<volume_id>', methods=['GET'])
def unread_volume(series_id, volume_id):
    kavita.set_volume_as_unread(series_id, volume_id)
    return jsonify(status="success")

@app.route('/api/server-settings', methods=['GET'])
def get_server_settings():
    """Get the current server settings"""
    logger.info("GET /api/server-settings - Retrieving server settings")
    settings = {
        "ip": kavita.ip,
        "username": kavita.username,
        "offline_mode": kavita.offline_mode,
        "logged_as": kavita.logged_as
    }
    logger.info(f"Current server settings: IP={settings['ip']}, Username={settings['username']}, Offline={settings['offline_mode']}, LoggedAs={settings['logged_as']}")
    return jsonify(settings)

@app.route('/api/server-settings', methods=['POST'])
def update_server_settings():
    """Update the server settings"""
    data = request.json
    logger.info(f"POST /api/server-settings - Received update request: {data}")
    
    # Get the values from the request
    new_ip = data.get('ip')
    new_username = data.get('username')
    new_password = data.get('password')
    
    # Validate the IP format
    if new_ip:
        if ':' in new_ip:
            host, port = new_ip.split(':')
            try:
                port_num = int(port)
                if port_num < 1 or port_num > 65535:
                    return jsonify(status="error", message="Invalid port number. Must be between 1 and 65535"), 400
            except ValueError:
                return jsonify(status="error", message="Invalid port format. Must be a number"), 400
    
    logger.info(f"Updating server settings - IP: {new_ip}, Username: {new_username}, Password: {'*****' if new_password else 'not changed'}")
    
    # Get current connection status before update
    old_status = not kavita.get_offline_mode()
    old_ip = kavita.get_kavita_ip()
    old_logged_as = kavita.logged_as
    
    logger.info(f"Current connection status before update - Connected: {old_status}, IP: {old_ip}, LoggedAs: {old_logged_as}")
    
    # Update the settings
    success, message = kavita.update_server_settings(
        new_ip=new_ip, 
        new_username=new_username, 
        new_password=new_password
    )
    
    # Get the current settings after update attempt
    current_settings = {
        "ip": kavita.ip,
        "username": kavita.username,
        "offline_mode": kavita.offline_mode,
        "logged_as": kavita.logged_as,
        "url": kavita.url
    }
    
    logger.info(f"Update result: {success}, {message}")
    logger.info(f"Current connection status after update - Connected: {not kavita.offline_mode}, IP: {kavita.ip}, LoggedAs: {kavita.logged_as}")
    logger.info(f"Current server settings after update: {current_settings}")
    
    if success:
        return jsonify(status="success", message=message, current_settings=current_settings)
    else:
        return jsonify(status="error", message=message, current_settings=current_settings), 400

@app.route('/api/logs')
def get_logs():
    """Return the current log buffer"""
    return jsonify({
        "logs": log_buffer,
        "count": len(log_buffer)
    })

if __name__ == '__main__':
    try:
        # Check if port is already in use
        logger.info("Starting Flask server on port 11337")
        app.run(host='127.0.0.1', port=11337, debug=False, threaded=True, use_reloader=False)
    except OSError as e:
        logger.error(f"Port 11337 is already in use. Please close any existing instances: {e}")
        sys.exit(1)
    except Exception as e:
        logger.error(f"Unexpected error starting Flask: {e}")
        sys.exit(1)
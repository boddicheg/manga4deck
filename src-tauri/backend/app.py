from flask import Flask, jsonify, send_file
from flask_cors import CORS
import sys
import os

from lib.db import *
from lib.kavita import *

IP = "192.168.5.73:5001"

g_root = os.path.dirname(os.path.abspath(__file__))

username = "boddicheg"
password = "dyd6ZNU.aby*mqd6fwd"
api_key = "8df0fde8-8229-464c-ae0c-fd58a1a35b11"

app = Flask(__name__)
kavita = KavitaAPI(IP, username, password, api_key)
CORS(app)

@app.route('/api/status')
def status():
    is_offline = kavita.get_offline_mode()
    ip = kavita.get_kavita_ip()
    logged = kavita.logged_as
    cache_size = get_cache_size()
    return jsonify(status=not is_offline, ip=ip, logged_as=logged, cache=cache_size)

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

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=11337, debug=True)
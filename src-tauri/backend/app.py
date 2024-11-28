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
    return jsonify(status=not is_offline)

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
    return jsonify(v)

@app.route('/api/volumes-cover/<volume>', methods=['GET'])
def volume_cover(volume):
    cover = kavita.get_volume_cover(str(volume))
    return send_file(cover, as_attachment=True)

@app.route('/api/picture/<chapter>/<page>', methods=['GET'])
def picture(chapter, page):
    image = kavita.get_picture(chapter, page)
    return send_file(image, as_attachment=True)

if __name__ == '__main__':
    app.run(port=11337, debug=True)
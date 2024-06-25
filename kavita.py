import requests
import json
import time 
import os
import hashlib
import threading
import atexit

from db import *

CACHE_FOLDER = "./cache"
DB_PATH = "cache.sqlite"

class KavitaAPI():
    def __init__(self, ip, username, password, api_key):
        self.ip = ip
        self.url = f"http://{ip}/api/"
        self.api_key = api_key
        self.offline_mode = False
        self.lock = threading.Lock()
        self.database = DBSession(DB_PATH)

        if not os.path.exists(CACHE_FOLDER):
            os.mkdir(CACHE_FOLDER)
        
        atexit.register(self.destuctor)

        try:
            response = requests.post(
                self.url + "Account/login", 
                json={
                    "username": username,
                    "password": password,
                    "apiKey": api_key
                }
            )
            
            if len(response.content.decode()) == 0:
                raise("[!] Authentification failed!")
            
            auth_data = json.loads(response.content)
            if "token" in auth_data:
                self.token = auth_data["token"]
                self.logged_as = auth_data["username"]
                print(f"Logged as {self.logged_as}")
            else:
                raise("[!] Authentification failed!")
        except:
            self.offline_mode = True
            self.token = ""
            self.logged_as = ""
            print("Now in offline mode")
        # --
        self.caching_series_queue = []
        self.caching_callback = None
        self.running = True
        self.caching_thread = threading.Thread(target=self.cache_serie_threaded)
        self.caching_thread.start()

        # Upload progress 
        self.upload_progress()

    def destuctor(self):
        with self.lock:
            self.running = False
    
    def get_kavita_ip(self):
        return self.ip
    
    def get_cached_count(self):
        return len(self.cache["series"])
    
    def get_offline_mode(self):
        return self.offline_mode

    def clear_manga_cache(self):
        files = os.listdir(CACHE_FOLDER)
        for file in files:
            file_path = os.path.join(CACHE_FOLDER, file)
            if os.path.isfile(file_path):
                os.remove(file_path)

        self.database.clean()

    #--------------------------------------------------------------------------
    # Caching whole serie
    #--------------------------------------------------------------------------
    def is_series_cached(self, id):
        return self.database.is_series_cached(id)
    
    def is_volume_cached(self, id):
        return self.database.is_volume_cached(id)
    
    def cache_serie_threaded(self):
        while self.running:
            if len(self.caching_series_queue) == 0:
                time.sleep(0.1)
                continue
            cached_serie = self.caching_series_queue[0]
            del self.caching_series_queue[0]
            
            print(f"Start caching serie {cached_serie['id']} ")
            serie = {
                "id": cached_serie["id"],
                "title": cached_serie["title"],
                "read": 0,
                "pages": 0
            }

            volumes = self.get_volumes(cached_serie["id"])
            for v in volumes:
                volume_id = v["volume_id"]
                pages = v["pages"]
                cid = v["chapter_id"]
                # Caching manga pictures
                for p in range(1, pages + 1):
                    with self.lock:
                        if not self.running:
                            return
                    self.get_picture(cid, p)
                # Caching volume
                with self.lock:
                    self.database.add_volumes({
                        "volume_id": volume_id,
                        "series_id": cached_serie["id"],
                        "chapter_id": cid,
                        "title": v["title"],
                        "read": v['read'],
                        "pages": v['pages']
                    })
                serie["read"] += v['read']
                serie["pages"] += v['pages']
                # Update UI
                if self.caching_callback:
                    self.caching_callback(v["title"])
            # Cache serie
            with self.lock:
                self.database.add_series(serie)
            print(f"Finised caching serie")

    def cache_serie(self, serie, callback):
        with self.lock:
            self.caching_series_queue.append(serie)
            self.caching_callback = callback

    #--------------------------------------------------------------------------
    def get_library(self):
        result = []
        if not self.offline_mode:
            response = requests.get(
                self.url + "library/libraries", 
                headers={
                    "Accept": "application/json",
                    "Authorization": f"Bearer {self.token}"
                }
            )
        
            library = json.loads(response.content)
            for e in library:
                row = {
                    "id": e["id"],
                    "title": e["name"]
                }
                result.append(row)
                # caching for future needs
                self.database.add_library(row)
            self.database.commit_changes()
        else:
            # load from cache:
            result = self.database.get_libraries()

        return result
    
    def get_series(self, parent):
        result = []
        if not self.offline_mode:
            response = requests.post(
                self.url + f"series/v2",
                json={
                    "statements": [
                        {
                            "comparison": 0,
                            "field": 19,
                            "value": f"{parent}"
                        }
                    ],
                    "combination": 1,
                    "limitTo": 0
                },
                headers={
                    "Accept": "application/json",
                    "Authorization": f"Bearer {self.token}"
                }
            )
            if len(response.content.decode()) > 0:
                series = json.loads(response.content)
                for e in series:
                    row = {
                        "id": e["id"],
                        "title": e["name"],
                        "read": int(e["pagesRead"]) * 100 / int(e["pages"]),
                        "pages": int(e["pages"])
                    }
                    result.append(row)
        else:
            result = self.database.get_series()

        return result
    
    def get_series_cover(self, series):
        filename = ""
        with self.lock:
            filename = self.database.search_series_cover(series)
        if len(filename) > 0:
            return filename
        
        url = self.url + f"image/series-cover?seriesId={series}&apiKey={self.api_key}"
        response = requests.get(
            url,
            headers={
                "Content-Type": "image/png",
            }
        )
        
        filename = CACHE_FOLDER + "/" + hashlib.md5(str(time.time()).encode()).hexdigest() + ".png"
        with open(filename, 'wb') as f:
            f.write(response.content)
        
        self.database.add_series_cover({
            "seriesId": series,
            "file": filename
        })
        self.database.commit_changes()

        return filename
    
    def get_volumes(self, parent):
        result = []
        if not self.offline_mode:
            response = requests.get(
                self.url + f"series/series-detail?seriesId={parent}",
                headers={
                    "Accept": "application/json",
                    "Authorization": f"Bearer {self.token}"
                }
            )

            if len(response.content.decode()) > 0:
                data = json.loads(response.content)
                for vol in data["volumes"]:
                    if len(vol["chapters"]) > 0:
                        chapter_id = vol["chapters"][0]['id']
                        row = {
                            "volume_id": vol['id'],
                            "chapter_id": chapter_id,
                            "series_id": parent,
                            "title": vol["name"],
                            "read": vol['pagesRead'],
                            "pages": vol['pages']
                        }
                        result.append(row)
        else:
            result = self.database.get_volumes(parent)
        
        return result
    
    def get_volume_cover(self, volume):
        filename = ""
        with self.lock:
            filename = self.database.search_volume_cover( volume)
        if len(filename) > 0:
            return filename

        url = self.url + f"image/volume-cover?volumeId={volume}&apiKey={self.api_key}"
        print(f"url: {url}")
        response = requests.get(
            url,
            headers={
                "Content-Type": "image/png",
            }
        )
        
        filename = CACHE_FOLDER + "/" + hashlib.md5(str(time.time()).encode()).hexdigest() + ".png"
        with open(filename, 'wb') as f:
            f.write(response.content)
            
        # caching
        self.database.add_volume_cover({
            "volumeId": volume,
            "file": filename
        })
        self.database.commit_changes()

        return filename
    
    def get_picture(self, id, page):
        # http://192.168.5.49:5001/api/reader/image?chapterId=1498&apiKey=8df0fde8-8229-464c-ae0c-fd58a1a35b11&page=3
        filename = ""
        with self.lock:
            filename = self.database.search_manga_pics(id, page)
        if len(filename) > 0:
            return filename

        url = self.url + f"reader/image?chapterId={id}&apiKey={self.api_key}&page={page}"
        response = requests.get(
            url,
            headers={
                "Content-Type": "image/png",
            }
        )
        filename = CACHE_FOLDER + "/" + hashlib.md5(str(time.time()).encode()).hexdigest() + ".png"
        
        with open(filename, 'wb') as f:
            f.write(response.content)

        with self.lock:
            self.database.add_manga_pic({
                "chapter_id": id, 
                "page": page, 
                "file": filename
            })

        return filename
    
    def save_progress(self, ids):
        url = self.url + f"reader/progress"
        if not self.offline_mode:
            requests.post(
                url,
                json = {
                    "libraryId": ids["library_id"],
                    "seriesId": ids["series_id"],
                    "volumeId": ids["volume_id"],
                    "chapterId": ids["chapter_id"],
                    "pageNum": ids["page"],
                } ,
                headers={
                    "Accept": "application/json",
                    "Authorization": f"Bearer {self.token}"
                }
            )
        else:
            self.database.add_progress(ids)
            self.database.set_volume_as_read(ids["volume_id"], ids["series_id"], ids["page"])
            self.database.set_series_read_pages(ids["series_id"], ids["page"])

    def upload_progress(self):
        if not self.offline_mode:
            for record in self.database.get_progress():
                self.save_progress(record)
            self.database.clean_progress()
            
    def set_volume_as_read(self, series_id, volume_id):
        url = self.url + f"reader/mark-volume-read"
        if not self.offline_mode:
            requests.post(
                url,
                json = {
                    "seriesId": series_id,
                    "volumeId": volume_id
                } ,
                headers={
                    "Accept": "application/json",
                    "Authorization": f"Bearer {self.token}"
                }
            )
        else:
            self.database.set_volume_as_read(volume_id, series_id)

    def update_server_library(self):
        url = self.url + f"library/scan-all"
        if not self.offline_mode:
            requests.post(
                url,
                json = {
                    "force": True
                } ,
                headers={
                    "Accept": "application/json",
                    "Authorization": f"Bearer {self.token}"
                }
            )
        
import requests
import json
from icecream import ic
import time 
import os
import hashlib
import threading
import atexit

CACHE_FOLDER = "./cache"

class KavitaAPI():
    def __init__(self, url, username, password, api_key):
        self.url = url
        self.api_key = api_key
        
        if not os.path.exists(CACHE_FOLDER):
            os.mkdir(CACHE_FOLDER)
        
        atexit.register(self.destuctor)

        self.cache_thumbnail = []
        self.cache_thumbnail_file = CACHE_FOLDER + "/cache_thumbnail.json"
        
        if not os.path.exists(self.cache_thumbnail_file):
            with open(self.cache_thumbnail_file, 'w') as f:
                f.write(json.dumps(self.cache_thumbnail, indent=4))
        else:
            with open(self.cache_thumbnail_file, 'r') as f:
                self.cache_thumbnail = json.load(f)
                
        self.cache_manga = []
        self.cache_manga_file = CACHE_FOLDER + "/cache_manga.json"
        
        if not os.path.exists(self.cache_manga_file):
            with open(self.cache_thumbnail_file, 'w') as f:
                f.write(json.dumps(self.cache_manga, indent=4))
        else:
            with open(self.cache_manga_file, 'r') as f:
                self.cache_manga = json.load(f)
                
        self.cache_series = []
        self.cache_series_file = CACHE_FOLDER + "/cache_series.json"
        
        if not os.path.exists(self.cache_series_file):
            with open(self.cache_thumbnail_file, 'w') as f:
                f.write(json.dumps(self.cache_series, indent=4))
        else:
            with open(self.cache_series_file, 'r') as f:
                self.cache_series = json.load(f)

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
        # --
        self.lock = threading.Lock()
        self.caching_series_queue = []
        self.caching_callback = None
        self.running = True
        self.caching_thread = threading.Thread(target=self.cache_serie_threaded)
        self.caching_thread.start()
        
    def destuctor(self):
        with self.lock:
            self.running = False
            self.caching_thread.join()

        with open(self.cache_thumbnail_file, 'w') as f:
            f.write(json.dumps(self.cache_thumbnail, indent=4))
        with open(self.cache_manga_file, 'w') as f:
            f.write(json.dumps(self.cache_manga, indent=4))
        with open(self.cache_series_file, 'w') as f:
            f.write(json.dumps(self.cache_series, indent=4))
    
    def clear_manga_cache(self):
        for e in self.cache_manga:
            if os.path.isfile(e["file"]):
                os.remove(e["file"])

        for e in self.cache_thumbnail:
            if os.path.isfile(e["file"]):
                os.remove(e["file"])
                
        self.cache_thumbnail = []
        self.cache_manga = []
        self.cache_series = []
        
    def cache_serie_threaded(self):
        print("cache_serie_threaded +")
        while self.running:
            if len(self.caching_series_queue) == 0:
                time.sleep(0.1)
                continue
            cached_serie = self.caching_series_queue[0]
            del self.caching_series_queue[0]
            print(f"Start caching serie {cached_serie} ")
            serie = {
                "serie_id": cached_serie,
                "volumes": []
            }
            self.cache_series.append(serie)
            volumes = self.get_volumes(cached_serie)
            # {'id': 1483, 'chapter_id': 2399, 'title': 'Volume 40\n(0/147)', 'read': 0, 'pages': 147}
            for v in volumes:
                cid = v["chapter_id"]
                pages = v["pages"]
                for p in range(1, pages + 1):
                    self.get_picture(cid, p)
                self.cache_series[-1]["volumes"].append(v["id"])
                # Update UI
                if self.caching_callback:
                    self.caching_callback()
            
            print(f"Finised caching serie {serie} ")

        print("cache_serie_threaded -")

    def cache_serie(self, serie, callback):
        with self.lock:
            self.caching_series_queue.append(serie)
            self.caching_callback = callback
        
    def search_in_cover_cache(self, key, value):
        for e in self.cache_thumbnail:
            if key in e.keys() and e[key] == value:
                return e["file"]
        return ""
    
    def search_in_serie_cache(self, sid, vid):
        for e in self.cache_series:
            if e["serie_id"] == sid:
                if not vid:
                    return True
                else:
                    for v in e["volumes"]:
                        if v == vid:
                            return True
        return False
    
    def store_in_cover_cache(self, key, value, filename):
        self.cache_thumbnail.append({
            key: value,
            "file": filename
        })
        
    def search_in_manga_cache(self, key1, value1, key2, value2):
        for e in self.cache_manga:
            if key1 in e.keys() and \
               key2 in e.keys() and \
               e[key1] == value1 and \
               e[key2] == value2:
                return e["file"]
        return ""
    
    def store_in_manga_cache(self, key1, value1, key2, value2, filename):
        self.cache_manga.append({
            key1: value1,
            key2: value2,
            "file": filename
        })

    def get_library(self):
        response = requests.get(
            self.url + "library", 
            headers={
                "Accept": "application/json",
                "Authorization": f"Bearer {self.token}"
            }
        )
        result = []
        library = json.loads(response.content)
        for e in library:
            result.append({
                "id": e["id"],
                "title": e["name"]
            })
        return result
    
    def get_series(self, parent):
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
        result = []
        if len(response.content.decode()) > 0:
            series = json.loads(response.content)
            for e in series:
                result.append({
                    "id": e["id"],
                    "title": e["name"],
                    "read": int(e["pagesRead"]) * 100 / int(e["pages"])
                })

        return result
    
    def get_serie_cover(self, serie):
        filename = self.search_in_cover_cache("seriesId", serie)
        if len(filename) > 0:
            return filename
        
        url = self.url + f"image/series-cover?seriesId={serie}&apiKey={self.api_key}"
        # print(f"url: {url}")
        response = requests.get(
            url,
            headers={
                "Content-Type": "image/png",
            }
        )
        
        filename = CACHE_FOLDER + "/" + hashlib.md5(str(time.time()).encode()).hexdigest() + ".png"
        
        with open(filename, 'wb') as f:
            f.write(response.content)
            
        self.store_in_cover_cache("seriesId", serie, filename)
        return filename
    
    def get_volume_cover(self, volume):
        filename = self.search_in_cover_cache("volumeId", volume)
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
            
        self.store_in_cover_cache("volumeId", volume, filename)
        return filename
        
    def get_volumes(self, parent):
        response = requests.get(
            self.url + f"series/series-detail?seriesId={parent}",
            headers={
                "Accept": "application/json",
                "Authorization": f"Bearer {self.token}"
            }
        )
        
        result = []
        if len(response.content.decode()) > 0:
            data = json.loads(response.content)
            # ic(data)
            for vol in data["volumes"]:
                if len(vol["chapters"]) > 0:
                    chapter_id = vol["chapters"][0]['id']
                    result.append({
                        "id": vol['id'],
                        "chapter_id": chapter_id,
                        "title": vol["name"] + f"\n({vol['pagesRead']}/{vol['pages']})",
                        "read": vol['pagesRead'],
                        "pages": vol['pages']
                    })
        
        return result
    
    def get_picture(self, id, page):
        # http://192.168.5.49:5001/api/reader/image?chapterId=1498&apiKey=8df0fde8-8229-464c-ae0c-fd58a1a35b11&page=3
        filename = self.search_in_manga_cache("chapterId", id, "page", page)
        if len(filename) > 0:
            return filename
        url = self.url + f"reader/image?chapterId={id}&apiKey={self.api_key}&page={page}"
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
        self.store_in_manga_cache("chapterId", id, "page", page, filename)
        return filename
    
    def save_progress(self, ids):
        url = self.url + f"reader/progress"
        try:
            requests.post(
                url,
                json = {
                    "libraryId": ids["libraryId"],
                    "seriesId": ids["seriesId"],
                    "volumeId": ids["volumeId"],
                    "chapterId": ids["chapterId"],
                    "pageNum": ids["pageNum"],
                } ,
                headers={
                    "Accept": "application/json",
                    "Authorization": f"Bearer {self.token}"
                }
            )
        except:
            pass
        
        
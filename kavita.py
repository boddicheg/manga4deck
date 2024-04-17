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
    def __init__(self, ip, username, password, api_key):
        self.ip = ip
        self.url = f"http://{ip}/api/"
        self.api_key = api_key
        self.offline_mode = False
        self.lock = threading.Lock()

        if not os.path.exists(CACHE_FOLDER):
            os.mkdir(CACHE_FOLDER)
        
        atexit.register(self.destuctor)

        self.cache = {}
        self.kv_cache_fields = {
            # Menu structure
            "library": CACHE_FOLDER + "/cache_library.json",
            "series": CACHE_FOLDER + "/cache_series.json",
            "volumes": CACHE_FOLDER + "/cache_volumes.json",
            # Manga previews in menu
            "serie_covers": CACHE_FOLDER + "/cache_serie_covers.json",
            "volume_covers": CACHE_FOLDER + "/cache_volume_covers.json",
            # Manga images
            "manga": CACHE_FOLDER + "/cache_manga.json",
            # Offline progress manga
            "progress": CACHE_FOLDER + "/cache_progress.json",
        }

        for k in self.kv_cache_fields.keys():
            self.cache[k] = []
            filename = self.kv_cache_fields[k]

            if not os.path.exists(filename):
                with open(filename, 'w') as f:
                    f.write(json.dumps(self.cache[k], indent=4))
            else:
                with open(filename, 'r') as f:
                    self.cache[k] = json.load(f)

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

        for k in self.kv_cache_fields.keys():
            filename = self.kv_cache_fields[k]
            with open(filename, 'w') as f:
                f.write(json.dumps(self.cache[k], indent=4))
    
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
        
        for k in self.kv_cache_fields.keys():
            self.cache[k] = []

    #--------------------------------------------------------------------------
    # Caching whole serie
    #--------------------------------------------------------------------------
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
                pages = v["pages"]
                cid = v["chapter_id"]
                for p in range(1, pages + 1):
                    with self.lock:
                        if not self.running:
                            return
                    self.get_picture(cid, p)
                volume_already_cached = False
                for vv in self.cache["volumes"]:
                    if vv["chapter_id"] == cid:
                        volume_already_cached = True
                        vv["read"] = v["read"]
                if not volume_already_cached:
                    self.cache["volumes"].append({
                        "id": v['id'],
                        "serie_id": cached_serie["id"],
                        "chapter_id": v["chapter_id"],
                        "title": v["title"],
                        "read": v['read'],
                        "pages": v['pages']
                    })
                serie["read"] += v['read']
                serie["pages"] += v['pages']
                # Update UI
                if self.caching_callback:
                    self.caching_callback()
            # Cache serie
            serie_already_cached = False
            for s in self.cache["series"]:
                if s["id"] == cached_serie['id']:
                    serie_already_cached = True
                    s["read"] = serie["read"]
            if not serie_already_cached:
                self.cache["series"].append(serie)

            print(f"Finised caching serie")

    def cache_serie(self, serie, callback):
        with self.lock:
            self.caching_series_queue.append(serie)
            self.caching_callback = callback
    
    #--------------------------------------------------------------------------
    # Search / store covers

    def search_in_cover_cache(self, cache_name, key, value):
        for e in self.cache[cache_name]:
            if key in e.keys() and e[key] == value:
                return e["file"]
        return ""

    #--------------------------------------------------------------------------
    # Search / store manga images

    def is_serie_cached(self, id):
        for e in self.cache["series"]:
            if e["id"] == id:
                return True
        return False
    
    def is_volume_cached(self, id):
        for e in self.cache["volumes"]:
            if e["id"] == id:
                return True
        return False

    def search_in_manga_cache(self, key1, value1, key2, value2):
        for e in self.cache["manga"]:
            if key1 in e.keys() and \
               key2 in e.keys() and \
               e[key1] == value1 and \
               e[key2] == value2:
                return e["file"]
        return ""
    
    def store_in_manga_cache(self, key1, value1, key2, value2, filename):
        self.cache["manga"].append({
            key1: value1,
            key2: value2,
            "file": filename
        })

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
                result.append({
                    "id": e["id"],
                    "title": e["name"]
                })
            # caching for future needs
            self.cache["library"] = result
        else:
            # load from cache:
            result = self.cache["library"] 

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
                    result.append({
                        "id": e["id"],
                        "title": e["name"],
                        "read": int(e["pagesRead"]) * 100 / int(e["pages"])
                    })
        else:
            result = self.cache["series"]

        return result
    
    def get_serie_cover(self, serie):
        filename = self.search_in_cover_cache("serie_covers", "seriesId", serie)
        if len(filename) > 0:
            return filename
        
        url = self.url + f"image/series-cover?seriesId={serie}&apiKey={self.api_key}"
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
        self.cache["serie_covers"].append({
            "seriesId": serie,
            "file": filename
        })

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
                        result.append({
                            "id": vol['id'],
                            "chapter_id": chapter_id,
                            "title": vol["name"],
                            "read": vol['pagesRead'],
                            "pages": vol['pages']
                        })
        else:
            volumes = self.cache["volumes"]
            for v in volumes:
                if parent == v['serie_id']:
                    result.append(v)
        
        return result
    
    def get_volume_cover(self, volume):
        filename = self.search_in_cover_cache("volume_covers", "volumeId", volume)
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
        self.cache["volume_covers"].append({
            "volumeId": volume,
            "file": filename
        })

        return filename
    
    def get_picture(self, id, page):
        # http://192.168.5.49:5001/api/reader/image?chapterId=1498&apiKey=8df0fde8-8229-464c-ae0c-fd58a1a35b11&page=3
        filename = self.search_in_manga_cache("chapterId", id, "page", page)
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
        self.store_in_manga_cache("chapterId", id, "page", page, filename)
        return filename
    
    def save_progress(self, ids):
        url = self.url + f"reader/progress"
        if not self.offline_mode:
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
        else:
            found = False
            for record in self.cache["progress"]:
                if record["libraryId"] == ids["libraryId"] and \
                    record["seriesId"] == ids["seriesId"] and \
                    record["volumeId"] == ids["volumeId"] and \
                    record["chapterId"] == ids["chapterId"]:
                    record["pageNum"] = ids["pageNum"]
                    found = True
            if not found:
                self.cache["progress"].append({
                        "libraryId": ids["libraryId"],
                        "seriesId": ids["seriesId"],
                        "volumeId": ids["volumeId"],
                        "chapterId": ids["chapterId"],
                        "pageNum": ids["pageNum"],
                    })
            # update in other cache arrays
            for record in self.cache["volumes"]:
                if record["id"] == ids["volumeId"] and \
                    record["chapter_id"] == ids["chapterId"]:
                    record["read"] = ids["pageNum"]
                    break

            for record in self.cache["series"]:
                if record["id"] == ids["seriesId"]:
                    record["read"] = ids["pageNum"]
                    break

    def upload_progress(self):
        if not self.offline_mode:
            for record in self.cache["progress"]:
                self.save_progress(record)
            self.cache["progress"] = []
            
    def set_volume_as_read(self, serie, volume):
        url = self.url + f"reader/mark-volume-read"
        if not self.offline_mode:
            requests.post(
                url,
                json = {
                    "seriesId": serie,
                    "volumeId": volume
                } ,
                headers={
                    "Accept": "application/json",
                    "Authorization": f"Bearer {self.token}"
                }
            )
        else:
            for record in self.cache["volumes"]:
                if record["serie_id"] == serie and \
                    record["id"] == volume:
                    record["read"] = record["pages"]
                    break
        
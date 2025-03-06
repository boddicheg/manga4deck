import requests
import json
import time 
import os
import sys
import hashlib
import threading
import atexit
import logging

from lib.db import *

import sys
import pathlib

# Create module logger
logger = logging.getLogger(__name__)

def get_datadir() -> pathlib.Path:

    """
    Returns a parent directory path
    where persistent application data can be stored.

    # linux: ~/.local/share
    # macOS: ~/Library/Application Support
    # windows: C:/Users/<USER>/AppData/Roaming
    """

    home = pathlib.Path.home()

    if sys.platform == "win32":
        return home / "AppData/Roaming"
    elif sys.platform == "linux":
        return home / ".local/share"
    elif sys.platform == "darwin":
        return home / "Library/Application Support"

def get_appdir_path(relative_path):
    datadir = get_datadir() / "manga4deck"
    
    try:
        datadir.mkdir(parents=True)
    except FileExistsError:
        pass
    
    datadir = datadir / relative_path
    # print(str(datadir))
    return str(datadir)

DB_PATH = get_appdir_path("cache.sqlite")
CACHE_FOLDER = get_appdir_path("cache")

def get_cache_size(delimiter = 1024 * 1024 * 1024):
    files = os.listdir(CACHE_FOLDER)
    size = 0
    for f in files:
        path = CACHE_FOLDER + "/" + f
        stats = os.stat(path)
        size += stats.st_size
    return size / delimiter

class KavitaAPI():
    def __init__(self, ip, username, password, api_key):
        self.database = DBSession(DB_PATH)
        
        # Check if we have saved settings in the database
        saved_ip = self.database.get_server_setting("server_ip")
        saved_username = self.database.get_server_setting("username")
        saved_password = self.database.get_server_setting("password")
        saved_api_key = self.database.get_server_setting("api_key")
        
        # Use saved settings if available, otherwise use provided values
        self.ip = saved_ip if saved_ip else ip
        self.username = saved_username if saved_username else username
        self.password = saved_password if saved_password else password
        self.api_key = saved_api_key if saved_api_key else api_key
        
        # Save settings to database if they're not already saved
        if not saved_ip:
            self.database.set_server_setting("server_ip", ip)
        if not saved_username:
            self.database.set_server_setting("username", username)
        if not saved_password:
            self.database.set_server_setting("password", password)
        if not saved_api_key and api_key:
            self.database.set_server_setting("api_key", api_key)
        
        # Add some initial logs to ensure we have something to display
        logger.info(f"Using server IP: {self.ip}")
        logger.info(f"Using username: {self.username}")
        logger.info(f"API Key: {'Set' if self.api_key else 'Not set'}")
        
        # Parse the IP and port
        if ':' in self.ip:
            host, port = self.ip.split(':')
            self.host = host
            self.port = port
            logger.info(f"Parsed host: {self.host}, port: {self.port}")
            self.url = f"http://{self.host}:{self.port}/api/"
        else:
            self.host = self.ip
            self.port = "5000"  # Default Kavita port
            logger.info(f"Using default port 5000 for host: {self.host}")
            self.url = f"http://{self.host}:{self.port}/api/"
        
        logger.info(f"Full API URL: {self.url}")
        
        self.offline_mode = False
        self.lock = threading.Lock()

        if not os.path.exists(CACHE_FOLDER):
            os.mkdir(CACHE_FOLDER)
        
        atexit.register(self.destuctor)

        try:
            logger.info(f"Attempting to connect to {self.url}")
            response = requests.post(
                self.url + "Account/login", 
                json={
                    "username": self.username,
                    "password": self.password,
                    "apiKey": self.api_key
                },
                timeout=10  # Add a timeout to prevent hanging
            )
            
            logger.info(f"Response status code: {response.status_code}")
            
            if len(response.content.decode()) == 0:
                raise Exception("[!] Authentication failed! Empty response")
            
            auth_data = json.loads(response.content)
            # logger.info(f"Auth response: {auth_data}")
            
            if "token" in auth_data:
                self.token = auth_data["token"]
                self.logged_as = auth_data["username"]
                logger.info(f"Logged in as {self.logged_as}")
            else:
                raise Exception("[!] Authentication failed! No token in response")
        except Exception as e:
            logger.error(f"Connection error: {str(e)}")
            self.offline_mode = True
            self.token = ""
            self.logged_as = ""
            logger.warning("Now in offline mode")
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
        with self.lock:
            self.database.clean()

    #--------------------------------------------------------------------------
    # Caching whole serie
    #--------------------------------------------------------------------------
    def is_series_cached(self, id):
        with self.lock:
            return self.database.is_series_cached(id)
    
    def is_volume_cached(self, id):
        with self.lock:
            return self.database.is_volume_cached(id)
    
    def cache_serie_threaded(self):
        while self.running:
            if len(self.caching_series_queue) == 0:
                time.sleep(0.1)
                continue
            cached_serie = self.caching_series_queue[0]
            del self.caching_series_queue[0]
            
            logger.info(f"Start caching serie {cached_serie['id']} ")
            serie = {
                "id": cached_serie["id"],
                "title": cached_serie["title"],
                "read": 0,
                "pages": 0
            }

            volumes = self.get_volumes(cached_serie["id"])
            for v in volumes:
                serie["read"] += v['read']
                serie["pages"] += v['pages']
            # Cache serie
            with self.lock:
                self.database.add_series(serie)

            for v in volumes:
                volume_id = v["volume_id"]
                pages = v["pages"]
                read = v['read']
                if pages == read:
                    continue
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
                # Update UI
                if self.caching_callback:
                    self.caching_callback(v["title"])

            logger.info(f"Finised caching serie")

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
            with self.lock:
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
            with self.lock:
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
            with self.lock:
                result = self.database.get_series()

        return result
    
    def get_series_cover(self, series):
        filename = ""
        with self.lock:
            filename = self.database.search_series_cover(series)
        if len(filename) > 0:
            return filename
        
        if not self.offline_mode:
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
            
            with self.lock:
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
            with self.lock:
                result = self.database.get_volumes(parent)
        
        return result
    
    def get_volume_cover(self, volume):
        filename = ""
        with self.lock:
            filename = self.database.search_volume_cover( volume)
        if len(filename) > 0:
            return filename

        if not self.offline_mode:
            url = self.url + f"image/volume-cover?volumeId={volume}&apiKey={self.api_key}"
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
                
            # caching
            with self.lock:
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
        
        if not self.offline_mode:
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
            response = requests.post(
                url,
                json = {
                    # "libraryId": ids["library_id"],
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
            with self.lock:
                self.database.add_progress(ids)
                self.database.set_volume_as_read(ids["volume_id"], ids["series_id"], ids["page"])
                self.database.set_series_read_pages(ids["series_id"], ids["page"])

    def upload_progress(self):
        if not self.offline_mode:
            with self.lock:
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
            with self.lock:
                self.database.set_volume_as_read(volume_id, series_id)

    def set_volume_as_unread(self, series_id, volume_id):
        url = self.url + f"reader/mark-volume-unread"
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

    def update_server_settings(self, new_ip=None, new_username=None, new_password=None, new_api_key=None):
        """Update the server settings and try to reconnect"""
        with self.lock:
            # Save old values for rollback if needed
            old_ip = self.ip
            old_host = getattr(self, 'host', '')
            old_port = getattr(self, 'port', '')
            old_url = self.url
            old_username = self.username
            old_password = self.password
            old_api_key = self.api_key
            old_token = self.token
            old_logged_as = self.logged_as
            old_offline_mode = self.offline_mode
            
            # Terminate existing connection
            logger.info(f"Terminating existing connection to {self.url}")
            self.token = ""
            self.logged_as = ""
            self.offline_mode = True
            
            # Update values if provided
            if new_ip:
                logger.info(f"Updating server IP from {self.ip} to {new_ip}")
                self.ip = new_ip
                
                # Parse the IP and port
                if ':' in new_ip:
                    host, port = new_ip.split(':')
                    self.host = host
                    self.port = port
                    logger.info(f"Parsed host: {self.host}, port: {self.port}")
                    self.url = f"http://{self.host}:{self.port}/api/"
                else:
                    self.host = new_ip
                    self.port = "5000"  # Default Kavita port
                    logger.info(f"Using default port 5000 for host: {self.host}")
                    self.url = f"http://{self.host}:{self.port}/api/"
                    
                logger.info(f"New API URL: {self.url}")
                self.database.set_server_setting("server_ip", new_ip)
            
            if new_username:
                logger.info(f"Updating username from {self.username} to {new_username}")
                self.username = new_username
                self.database.set_server_setting("username", new_username)
                
            if new_password:
                logger.info("Updating password")
                self.password = new_password
                self.database.set_server_setting("password", new_password)
            
            if new_api_key:
                logger.info("Updating API key")
                self.api_key = new_api_key
                self.database.set_server_setting("api_key", new_api_key)
            
            # Try to reconnect
            try:
                logger.info(f"Attempting to connect to {self.url}")
                response = requests.post(
                    self.url + "Account/login", 
                    json={
                        "username": self.username,
                        "password": self.password,
                        "apiKey": self.api_key
                    },
                    timeout=10  # Add a timeout to prevent hanging
                )
                
                logger.info(f"Response status code: {response.status_code}")
                
                if response.status_code != 200:
                    logger.error(f"Error response: {response.text}")
                    raise Exception(f"Authentication failed! Status code: {response.status_code}")
                
                if len(response.content.decode()) == 0:
                    raise Exception("Authentication failed! Empty response")
                
                auth_data = json.loads(response.content)
                logger.info(f"Auth response: {auth_data}")
                
                if "token" in auth_data:
                    self.token = auth_data["token"]
                    self.logged_as = auth_data["username"]
                    self.offline_mode = False
                    logger.info(f"Reconnected as {self.logged_as}")
                    
                    # Clear any cached data to force refresh with new server
                    logger.info(f"Clearing cached data to refresh with new server")
                    self.caching_series_queue = []
                    
                    return True, "Connected successfully"
                else:
                    raise Exception("Authentication failed! No token in response")
            except Exception as e:
                error_msg = f"Failed to connect to server: {str(e)}"
                logger.error(error_msg)
                # Revert to old values if connection fails
                self.ip = old_ip
                self.host = old_host
                self.port = old_port
                self.url = old_url
                self.username = old_username
                self.password = old_password
                self.api_key = old_api_key
                self.token = old_token
                self.logged_as = old_logged_as
                self.offline_mode = old_offline_mode
                return False, error_msg

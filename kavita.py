import requests
import json
from icecream import ic
import time 
import os
import hashlib

CACHE_FOLDER = "./cache"

class KavitaAPI():
    def __init__(self, url, username, password, api_key):
        self.url = url
        self.api_key = api_key
        response = requests.post(
            self.url + "Account/login", 
            json={
                "username": username,
                "password": password,
                "apiKey": api_key
            }
        )
        
        if not os.path.exists("./cache"):
            os.mkdir("./cache")
        else:
            for filename in os.listdir(CACHE_FOLDER):
                file_path = os.path.join(CACHE_FOLDER, filename)
                try:
                    if os.path.isfile(file_path) or os.path.islink(file_path):
                        os.unlink(file_path)
                except Exception as e:
                    print('[!] Failed to delete %s. Reason: %s' % (file_path, e))
        
        if len(response.content.decode()) == 0:
            raise("[!] Authentification failed!")
        
        auth_data = json.loads(response.content)
        if "token" in auth_data:
            self.token = auth_data["token"]
            self.logged_as = auth_data["username"]
            print(f"Logged as {self.logged_as}")
        else:
            raise("[!] Authentification failed!")

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
                    "title": e["name"]
                })

        return result
    
    def get_serie_cover(self, serie):
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
        
        return filename
    
    def get_volume_cover(self, serie):
        url = self.url + f"image/volume-cover?volumeId={serie}&apiKey={self.api_key}"
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
            for vol in data["volumes"]:
                result.append({
                    "id": vol["id"],
                    "title": vol["name"] + f"({vol['pagesRead']}/{vol['pages']})",
                    "read": vol['pagesRead'],
                    "pages": vol['pages']
                })
        
        return result
            
        
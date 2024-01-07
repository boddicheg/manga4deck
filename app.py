import tkinter
from tkinter import ttk
import tkinter.messagebox
import customtkinter
from PIL import Image, ImageTk
import requests 
# import xml.etree.ElementTree as ET
from lxml import etree

import feedparser

customtkinter.set_appearance_mode("Dark")  # Modes: "System" (standard), "Dark", "Light"
customtkinter.set_default_color_theme("dark-blue")  # Themes: "blue" (standard), "green", "dark-blue"

URL = "http://192.168.5.49:5001/api/opds"
API_KEY = "8df0fde8-8229-464c-ae0c-fd58a1a35b11"
PREFIX = "{http://www.w3.org/2005/Atom}"
THUMB_PATH = "./thumbnail.png"

class OPDS:
    def __init__(self, url, api_key):
        self.root_url = url + "/" + api_key
    
    def get_entries(self, uri):
        print(f"\n->> GET {self.root_url}{uri}\n")
        parsed_feed = feedparser.parse(self.root_url + uri)

        entries = []
        for i, entry in enumerate(parsed_feed.entries, start=1):
            entries.append({
                "title": entry.title,
                "link": entry.links[0].href,
                "id": entry.id
            })
            # print(f"\nEntry {i}:")
            print(entry)
        return entries
    
class CTkLabelEx(customtkinter.CTkLabel):
    def __init__(self, master, text, text_color, image):
        customtkinter.CTkLabel.__init__(self, master=master, text=text, text_color=text_color, image=image)
        self.metadata = {}
    
    def set_metadata(self, md):
        self.metadata = md
        
    def get_metadata(self):
        return self.metadata

class App(customtkinter.CTk):
    def __init__(self):
        super().__init__()
        
        self.title("Manga4Deck")
        self.geometry(f"{1280}x{720}")
        
        self.opds = OPDS(URL, API_KEY)
        root_entries = self.opds.get_entries('/')

        self.draw_tites(root_entries)
        self.history = ["/"]
            
    def draw_tites(self, tiles):
        for i, entry in enumerate(tiles, start=0):
            col = int(i % 8)
            row = int(i / 8)
            
            tile = customtkinter.CTkFrame(self.master, width=150, height=200, corner_radius=5,)
            tile.grid(row=row, column=col, padx=5, pady=5)
            
            # add image and title
            img = ImageTk.PhotoImage(Image.open(THUMB_PATH).resize((150, 200)))
            label = CTkLabelEx(tile, text=entry["title"], text_color='white', image=img)
            label.set_metadata(entry)
            label.place(relx=.5, rely=.5, anchor='center')
            label.bind("<Double-1>", self.OnDoubleClick)
            label.pack()
            
            self.bind("<BackSpace>", self.back_in_history)

    def clean_master(self):
        for child in self.winfo_children():
            child.destroy()
        
    def back_in_history(self, event):
        print("->> BACK <<-")
        if len(self.history) > 1:
            self.history.pop()
            last_url = self.history[-1]
            self.clean_master()
            sub_entries = self.opds.get_entries(last_url)
            self.draw_tites(sub_entries)
        
    def OnDoubleClick(self, event):
        print(event.widget.master.get_metadata())
        item_metadata = event.widget.master.get_metadata()

        if "link" in item_metadata.keys():
            last_uri_section = item_metadata["link"].rsplit('/', 1)[-1]
            last_in_history = self.history[-1]
            full_url = last_in_history + last_uri_section + "/"
            sub_entries = self.opds.get_entries(full_url)
            self.history.append(full_url)
            self.clean_master()
            self.draw_tites(sub_entries)
            
        print("history no is ", self.history)
        
if __name__ == "__main__":
    app = App()
    app.mainloop()
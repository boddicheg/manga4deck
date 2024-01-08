import tkinter
from tkinter import ttk
import tkinter.messagebox
import customtkinter
from PIL import Image, ImageTk
from kavita import * 
from enum import Enum
import io

import feedparser

customtkinter.set_appearance_mode("Dark")  # Modes: "System" (standard), "Dark", "Light"
customtkinter.set_default_color_theme("dark-blue")  # Themes: "blue" (standard), "green", "dark-blue"

URL = "http://192.168.5.49:5001/api/"
API_KEY = "8df0fde8-8229-464c-ae0c-fd58a1a35b11"
PREFIX = "{http://www.w3.org/2005/Atom}"
THUMB_PATH = "./assets/thumbnail.png"
CACHE_PATH = "./cache"

WIDTH = 1280
HEIGHT = 720

username = "boddicheg"
password = "dyd6ZNU.aby*mqd6fwd"
api_key = "8df0fde8-8229-464c-ae0c-fd58a1a35b11"

class EntryType(Enum):
    SHELF = 1
    LIBRARY = 2
    SERIE = 3
    VOLUME = 4
    PICTURE = 5
    
class CTkLabelEx(customtkinter.CTkLabel):
    def __init__(self, master, text, text_color, image = None, fg_color="transparent"):
        customtkinter.CTkLabel.__init__(self, master=master, text=text, text_color=text_color, image=image, fg_color=fg_color)
        self.metadata = {}
    
    def set_metadata(self, md):
        self.metadata = md
        
    def get_metadata(self):
        return self.metadata

class App(customtkinter.CTk):
    def __init__(self):
        super().__init__()
        
        self.title("Manga4Deck")
        self.geometry(f"{WIDTH}x{HEIGHT}")
        
        self.kavita = KavitaAPI(URL, username, password, api_key)
        self.history = [{ "type": EntryType.SHELF, "parent_id": -1}]
        self.draw()

        self.bind("<BackSpace>", self.back_in_history)
        
    def draw(self):
        entries = []
        state = self.history[-1]["type"]
        parent = self.history[-1]["parent_id"]
        if state == EntryType.SHELF:
            entries.append({
                "id": -1,
                "title": "Kavita"
            })
        elif state == EntryType.LIBRARY:
            entries = self.kavita.get_library()
        elif state == EntryType.SERIE:
            entries = self.kavita.get_series(parent)
        elif state == EntryType.VOLUME:
            entries = self.kavita.get_volumes(parent)
            
        for i, entry in enumerate(entries, start=0):
            col = int(i % 8)
            row = int(i / 8)
            
            tile = customtkinter.CTkFrame(self.master, width=150, height=200, corner_radius=5,)
            tile.grid(row=row, column=col, padx=5, pady=5)
            
            # add image and title
            if state == EntryType.SERIE:
                filepath = self.kavita.get_serie_cover(entry["id"])
                img = ImageTk.PhotoImage(Image.open(filepath).resize((150, 200))) 
            elif state == EntryType.VOLUME:
                filepath = self.kavita.get_volume_cover(entry["id"])
                img = ImageTk.PhotoImage(Image.open(filepath).resize((150, 200))) 
            else:
                img = ImageTk.PhotoImage(Image.open(THUMB_PATH).resize((150, 200)))
            title = entry["title"][0:14] + "..." if len(entry["title"]) > 13 else entry["title"]
            label = CTkLabelEx(tile, text=title, text_color='white', fg_color="black" )
            label.configure(image=img)# = img
            label.set_metadata(entry)
            label.place(relx=.5, rely=.5, anchor='center')
            label.bind("<Double-1>", self.OnDoubleClick)
            label.pack()
            
    def clean_master(self):
        for child in self.winfo_children():
            child.destroy()
        
    def back_in_history(self, event):
        if len(self.history) > 1:
            self.history.pop()
            self.clean_master()
            self.draw()
        
    def OnDoubleClick(self, event):
        metadata = event.widget.master.get_metadata()
        print(metadata)
        
        last_in_history = self.history[-1]["type"]
        
        if last_in_history == EntryType.SHELF:
            self.history.append({ "type": EntryType.LIBRARY, "parent_id": -1})
        elif last_in_history == EntryType.LIBRARY:
            self.history.append({ "type": EntryType.SERIE, "parent_id": metadata["id"]})
        elif last_in_history == EntryType.SERIE:
            self.history.append({ "type": EntryType.VOLUME, "parent_id": metadata["id"]})
        
        self.clean_master()
        self.draw()
            
        print("history no is ", self.history)
        
if __name__ == "__main__":
    app = App()
    app.mainloop()
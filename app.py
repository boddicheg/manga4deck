import tkinter as tk
from tkinter import ttk
import tkinter.messagebox
import customtkinter
from PIL import Image, ImageTk
from kavita import * 
from enum import IntEnum

customtkinter.set_appearance_mode("Dark")  # Modes: "System" (standard), "Dark", "Light"
customtkinter.set_default_color_theme("dark-blue")  # Themes: "blue" (standard), "green", "dark-blue"

class EntryType(IntEnum):
    CLEAN_CACHE = -3
    SHELF_CACHED = -2
    SHELF = 1
    LIBRARY = 2
    SERIE = 3
    VOLUME = 4
    PICTURE = 5

IP = "192.168.5.49:5001"
THUMB_PATH = "./assets/thumbnail.png"
SETTINGS_PATH = "./assets/settings.png"
CACHE_PATH = "./cache"

username = "boddicheg"
password = "dyd6ZNU.aby*mqd6fwd"
api_key = "8df0fde8-8229-464c-ae0c-fd58a1a35b11"

WIDTH = 1280
HEIGHT = 800

PIC_WIDTH = 496
PIC_HEIGHT = 800

BIG_PIC_WIDTH = 960
BIG_PIC_HEIGHT = 800

# default is in gbytes
def cache_size(delimiter = 1024 * 1024 * 1024):
    files = os.listdir(CACHE_PATH)
    size = 0
    for f in files:
        if ".json" in f:
            continue
        path = CACHE_PATH + "/" + f
        stats = os.stat(path)
        size += stats.st_size
    return size / delimiter
class CTkLabelEx(customtkinter.CTkLabel):
    def __init__(self, master, width, height, text, text_color, image = None, fg_color="black", compound="center"):
        customtkinter.CTkLabel.__init__(self, master=master, 
                                        text=text, 
                                        text_color=text_color, 
                                        image=image, 
                                        fg_color=fg_color, 
                                        width=width, 
                                        height=height,
                                        compound=compound)
        self.metadata = {}
    
    def set_metadata(self, md):
        self.metadata = md
        
    def get_metadata(self):
        return self.metadata

class App(customtkinter.CTk):
    def __init__(self):
        super().__init__()

        self.grid_rowconfigure(0, weight=1)
        self.grid_columnconfigure(0, weight=1)

        # Main frame
        self.main_frame = customtkinter.CTkScrollableFrame(self, width=WIDTH, height=HEIGHT)
        self.main_frame.grid(row=0, column=0, padx=0, pady=0)
        
        self.title("Manga4Deck")
        self.geometry(f"{WIDTH}x{HEIGHT}")
        
        self.kavita = KavitaAPI(IP, username, password, api_key)
        self.history = [{ "type": EntryType.SHELF, "parent_id": -1}]
        self.draw()

        self.bind("<BackSpace>", self.back_in_history)
        self.bind("<Left>", self.previous_page)
        self.bind("<Right>", self.next_page)
        self.bind("<Down>", self.scroll_down)
        self.bind("<Up>", self.scroll_up)
        self.bind("<Return>", self.enter_to)
        self.bind("<F2>", self.cache_serie)
        
        self.focused = 0
        # Call destructor on window closing
        self.protocol("WM_DELETE_WINDOW", self.destructor)

    def destructor(self):
        self.kavita.running = False
        self.destroy()

    def update(self):
        self.after(100, self.draw)
        
    def cache_serie(self, event):
        last_in_history = self.history[-1]
        if last_in_history["type"] == EntryType.VOLUME:
            caching_serie = last_in_history["parent_id"]
            self.kavita.cache_serie(caching_serie, self.update)

    def scroll_down(self, event):
        self.main_frame._parent_canvas.yview_scroll(1, "units")
        
    def scroll_up(self, event):
        self.main_frame._parent_canvas.yview_scroll(-1, "units")

    def format_tile_desc(self, label, desc):
        title = label[0:14] + "..." if len(label) > 13 else label
        if len(desc) > 0:
            title += f"\n{desc}"
        return title 

    def draw_tile(self, entry, row, col):
        text = self.format_tile_desc(entry["title"], entry["description"])

        label = CTkLabelEx(self.main_frame, 
                        text=text, 
                        text_color=entry["text_color"], 
                        fg_color=entry["fg_color"],
                        width=150, 
                        height=200,
                        compound="bottom")
        
        img = ImageTk.PhotoImage(Image.open(entry["thumbnail"]).resize((150, 200)))
        label.configure(image=img)
        label.set_metadata(entry)
        label.bind("<Button-1>", self.OnSingleClick)
        label.bind("<FocusIn>", self.OnFocusIn)
        label.bind("<FocusOut>", self.OnFocusOut)
        label.grid(row=row, column=col, padx=5, pady=5) 
    
    def draw_shelf(self):
        sections = [{
            "id": -1,
            "title": "Kavita",
            "thumbnail": THUMB_PATH,
            "description": self.kavita.get_kavita_ip(),
            "fg_color": "black",
            "text_color": "white"
        }, {
            "id": -2,
            "title": "Cached Manga",
            "thumbnail": THUMB_PATH,
            "description": f"{self.kavita.get_cached_count()} series",
            "fg_color": "yellow",
            "text_color": "black"
        }, {
            "id": -3,
            "title": "Clean Cache",
            "thumbnail": SETTINGS_PATH,
            "description": f"Size: {cache_size():.1f}Gb",
            "fg_color": "black",
            "text_color": "white"
        }]

        for i, entry in enumerate(sections, start=0):
            col = int(i % 8)
            row = int(i / 8)
            self.draw_tile(entry, row, col)

    def draw(self):
        tiles = []
        state = self.history[-1]["type"]
        parent = self.history[-1]["parent_id"]

        # Start page
        if state == EntryType.SHELF:
            self.draw_shelf()
            return

        if state == EntryType.LIBRARY:
            entries = self.kavita.get_library()
            for e in entries:
                tiles.append({
                    "id": 2,
                    "title": e["title"],
                    "thumbnail": THUMB_PATH,
                    "description": "",
                    "fg_color": "black",
                    "text_color": "white"
                })
        elif state == EntryType.SERIE:
            entries = self.kavita.get_series(parent)
            for e in entries:
                t = {
                    "id": e["id"],
                    "title": e["title"],
                    "thumbnail": self.kavita.get_serie_cover(e["id"]),
                    "description": f"Read: {e['read']:.1f}%" ,
                    "fg_color": "black",
                    "text_color": "white"
                }
                if self.kavita.search_in_serie_cache(e["id"], None):
                    t["fg_color"] = "yellow"
                    t["text_color"] = "black"
                if e["read"] == 100:
                    t["fg_color"] = "green"
                    t["text_color"] = "white"
                tiles.append(t)
        elif state == EntryType.VOLUME:
            entries = self.kavita.get_volumes(parent)
            for e in entries:
                t = {
                    "id": e["id"],
                    "title": e["title"],
                    "thumbnail": self.kavita.get_volume_cover(e["id"]),
                    "description": "" ,
                    "fg_color": "black",
                    "text_color": "white",
                    "chapter_id": e["chapter_id"],
                    "read": e["read"],
                    "pages": e["pages"]
                }
                if self.kavita.search_in_serie_cache(e["id"], None):
                    t["fg_color"] = "yellow"
                    t["text_color"] = "black"
                if e["pages"] == e["read"]:
                    t["fg_color"] = "green"
                    t["text_color"] = "white"
                tiles.append(t)

        # Drawing things
        if state == EntryType.PICTURE:            
            self.history[-1]["read"] -= 1
            self.next_page(None)
        else:
            for i, entry in enumerate(tiles, start=0):
                col = int(i % 8)
                row = int(i / 8)
                self.draw_tile(entry, row, col)
                
    def draw_pic(self, filepath, left = False, first_small = False):
        image = Image.open(filepath)
        row = 0
        col = 0
        padx = (170, 0)
        factor = BIG_PIC_WIDTH / image.width
        w = int(image.width * factor)
        h = int(image.height * factor)
        self.main_frame._parent_canvas.yview_scroll(-100, "units")
            
        img = ImageTk.PhotoImage(image.resize((w, h)))
        label = CTkLabelEx(self.main_frame, text="", text_color='white', fg_color="black", width=w, height=h)
        label.configure(image=img)
        label.place(relx=.5, rely=.5, anchor='center')
        label.grid(row=row, column=col, padx=padx, pady=0)

    def clean_master(self):
        for child in self.main_frame.winfo_children():
            child.destroy()
        
    def back_in_history(self, event):
        if len(self.history) > 1:
            self.history.pop()
            self.clean_master()
            self.main_frame.destroy()
            self.main_frame = customtkinter.CTkScrollableFrame(self, width=WIDTH, height=HEIGHT)
            self.main_frame.grid(row=0, column=0, padx=0, pady=0)
            self.draw()

    def OnSingleClick(self, event):
        self.focused = 0
        metadata = event.widget.master.get_metadata()
        last_in_history = self.history[-1]["type"]

        print(metadata)

        if last_in_history == EntryType.SHELF and metadata["id"] == int(EntryType.CLEAN_CACHE):
            self.kavita.clear_manga_cache()
            self.update()
        elif last_in_history == EntryType.SHELF:
            self.history.append({ "type": EntryType.LIBRARY, "parent_id": -1})
        elif last_in_history == EntryType.LIBRARY:
            self.history.append({ "type": EntryType.SERIE, "parent_id": metadata["id"]})
        elif last_in_history == EntryType.SERIE:
            self.history.append({ "type": EntryType.VOLUME, "parent_id": metadata["id"]})
        elif last_in_history == EntryType.VOLUME:
            self.history.append({ 
                                 "type": EntryType.PICTURE, 
                                 "parent_id": metadata["id"],
                                 "chapter_id": metadata["chapter_id"],
                                 "read": metadata["read"], 
                                 "pages": metadata["pages"]
                                 })
        self.clean_master()
        self.draw()
        
    def OnFocusIn(self, event):
        event.widget.master.configure(text_color="red")

    def OnFocusOut(self, event):
        event.widget.master.configure(text_color="white")
        
    def is_small_pic(self, filepath):
        image = Image.open(filepath)
        return (image.width / image.height) < 1.0

    def previous_page(self, event):
        last_in_history = self.history[-1]["type"]
        
        if last_in_history == EntryType.PICTURE:
            self.clean_master()
            chapter_id = self.history[-1]["chapter_id"]
            current_page = self.history[-1]["read"]
            if current_page > 0:
                self.history[-1]["read"] -= 1
                filepath = self.kavita.get_picture(chapter_id, self.history[-1]["read"])
                self.draw_pic(filepath, False, False)
        else:
            if self.focused - 1 >= 0:
                self.focused -= 1
                self.main_frame.winfo_children()[self.focused].focus()
                
    def next_page(self, event):
        last_in_history = self.history[-1]["type"]
        if last_in_history == EntryType.PICTURE:
            self.clean_master()
            chapter_id = self.history[-1]["chapter_id"]
            current_page = self.history[-1]["read"]
            if current_page <= self.history[-1]["pages"]:
                print(f"Current position: {chapter_id}, page {current_page + 1}")
                self.history[-1]["read"] += 1
                filepath = self.kavita.get_picture(chapter_id, self.history[-1]["read"])
                self.draw_pic(filepath, False, False)
                # Save Progress
                progress = {
                    "libraryId": 0,
                    "seriesId": 0,
                    "volumeId": 0,
                    "chapterId": 0,
                    "pageNum": 0,
                }
                for e in self.history:
                    if e["type"] == EntryType.SERIE:
                        progress["libraryId"] = e["parent_id"]
                    if e["type"] == EntryType.VOLUME:
                        progress["seriesId"] = e["parent_id"]
                    if e["type"] == EntryType.PICTURE:
                        progress["volumeId"] = e["parent_id"]
                        progress["chapterId"] = e["chapter_id"]
                        progress["pageNum"] = e["read"]

                self.kavita.save_progress(progress)
        else:
            count = len(self.main_frame.winfo_children())
            if self.focused < count:
                self.main_frame.winfo_children()[self.focused].focus()
                self.focused += 1

    def enter_to(self, event):
        self.main_frame.focus_get().event_generate("<Button-1>")

if __name__ == "__main__":
    app = App()
    app.mainloop()
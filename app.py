import tkinter as tk
from tkinter import ttk ,Toplevel, Label
import tkinter.messagebox
import customtkinter
from PIL import Image, ImageTk
from kavita import * 
from enum import IntEnum

customtkinter.set_appearance_mode("Dark")  # Modes: "System" (standard), "Dark", "Light"
customtkinter.set_default_color_theme("dark-blue")  # Themes: "blue" (standard), "green", "dark-blue"

class EntryType(IntEnum):
    EXIT = -5
    ENABLE_OFFLINE_MODE = -4
    UPDATE_SERVER_LIB = -3
    CLEAN_CACHE = -2
    SHELF = 1
    LIBRARY = 2
    SERIE = 3
    VOLUME = 4
    PICTURE = 5

IP = "192.168.5.73:5001"
THUMB_IMAGE_PATH = "./assets/thumbnail.png"
SETTINGS_IMAGE_PATH = "./assets/settings.png"
EXIT_IMAGE_PATH = "./assets/exit.jpg"
OFFLINE_IMAGE_PATH = "./assets/offline.jpg"
CACHE_IMAGE_PATH = "./assets/cache.jpg"
CACHE_PATH = "./cache"

username = "boddicheg"
password = "dyd6ZNU.aby*mqd6fwd"
api_key = "8df0fde8-8229-464c-ae0c-fd58a1a35b11"

WIDTH = 1280
HEIGHT = 800

PIC_WIDTH = 960
PIC_HEIGHT = 800

TOAST_W = 300
TOAST_H = 30

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

    def set_text_color(self, color):
        self.configure(text_color=color)

    def set_fg_color(self, color):
        self.configure(fg_color=color)
    
    def set_text(self, text):
        self.configure(text=text)

class App(customtkinter.CTk):
    def __init__(self):
        super().__init__()
        
        self.app_running = True
        self.focused_selection_history = []
        self.focused = 0
        self.toasts = []

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
        self.bind("<F1>", self.set_volume_as_read)
        self.bind("<F2>", self.cache_serie)
        
        # Call destructor on window closing
        self.protocol("WM_DELETE_WINDOW", self.destructor)

    def destructor(self):
        self.app_running = False
        self.kavita.running = False
        self.destroy()

    def toast(self, message, alive = 5000, id_from_bottom = 1):
        win = Toplevel(self)
        win.wm_overrideredirect(True)
        win.attributes('-topmost', 'true')
        Label(win, text=message).pack()
        
        w = 20 + len(message) * 7
        
        x = self.winfo_rootx() + WIDTH - w - 25
        y = self.winfo_rooty() + HEIGHT - TOAST_H - (TOAST_H) * id_from_bottom 
        win.geometry("%dx%d+%d+%d" % (w, TOAST_H, x, y))
        self.after(alive, win.destroy)
        return win
        
    def draw_toast(self, message):
        self.toasts = list(filter(lambda x: (x.winfo_exists()), self.toasts))
        pos = len(self.toasts)
        t = self.toast(message, 5000, pos)
        self.toasts.append(t)

    def update(self, message=None):
        self.clean_master()
        self.after(100, self.draw)
        self.after(100, self.set_focus_on)
        if message:
            self.draw_toast(f"{message} has been cached")
        
    def cache_serie(self, event):
        last_in_history = self.history[-1]
        if last_in_history["type"] == EntryType.VOLUME:
            caching_serie = last_in_history["parent_id"]
            caching_title = last_in_history["title"]
            self.draw_toast(f"Start caching {caching_title}")
            self.kavita.cache_serie({ "id": caching_serie, "title": caching_title}, self.update)

    def scroll_down(self, event):
        self.main_frame._parent_canvas.yview_scroll(1, "units")
        
    def scroll_up(self, event):
        if self.main_frame._parent_canvas.yview() != (0.0, 1.0):
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
            "thumbnail": THUMB_IMAGE_PATH,
            "description": self.kavita.get_kavita_ip() if not self.kavita.get_offline_mode() else "Offline",
            "fg_color": "black" if not self.kavita.get_offline_mode() else "yellow",
            "text_color": "white" if not self.kavita.get_offline_mode() else "black"
        }, {
            "id": -2,
            "title": "Clean Cache",
            "thumbnail": CACHE_IMAGE_PATH,
            "description": f"Size: {cache_size():.1f}Gb" if not self.kavita.get_offline_mode() else "[Disabled]",
            "fg_color": "black",
            "text_color": "white"
        }, {
            "id": -3,
            "title": "Update lib",
            "thumbnail": CACHE_IMAGE_PATH,
            "description": "Server Kavita",
            "fg_color": "black",
            "text_color": "white"
        }, {
            "id": -4,
            "title": "Offline Mode",
            "thumbnail": OFFLINE_IMAGE_PATH,
            "description": "Only cached available",
            "fg_color": "black" if not self.kavita.get_offline_mode() else "yellow",
            "text_color": "white" if not self.kavita.get_offline_mode() else "black"
        }, {
            "id": -5,
            "title": "Exit",
            "thumbnail": EXIT_IMAGE_PATH,
            "description": "Close app",
            "fg_color": "black",
            "text_color": "white"
        }]

        for i, entry in enumerate(sections, start=0):
            col = int(i % 8)
            row = int(i / 8)
            self.draw_tile(entry, row, col)
        
        self.after(100, lambda: self.main_frame.winfo_children()[0].focus())

    def reset_scroll(self):
        self.main_frame._parent_canvas.yview_scroll(-100, "units")

    def draw(self):
        if not self.app_running:
            return

        tiles = []
        state = self.history[-1]["type"]
        parent = self.history[-1]["parent_id"]

        # Start page
        if state == EntryType.SHELF:
            self.draw_shelf()

        if state == EntryType.LIBRARY:
            entries = self.kavita.get_library()
            for e in entries:
                tiles.append({
                    "id": 2,
                    "title": e["title"],
                    "thumbnail": THUMB_IMAGE_PATH,
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
                    "thumbnail": self.kavita.get_series_cover(e["id"]),
                    "description": f"Read: {e['read']:.1f}%" ,
                    "fg_color": "black",
                    "text_color": "white"
                }

                if self.kavita.is_serie_cached(e["id"]):
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
                    "description":  f"({e['read']}/{e['pages']})" ,
                    "fg_color": "black",
                    "text_color": "white",
                    "chapter_id": e["chapter_id"],
                    "read": e["read"],
                    "pages": e["pages"]
                }
                if self.kavita.is_volume_cached(e["id"]):
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
            # Set Focus
            self.set_focus_on()
                
    def draw_pic(self, filepath):
        image = Image.open(filepath)
        row = 0
        col = 0
        padx = (0, 0)
        offset = 20
        factor = (WIDTH - offset) / image.width
        w = int(image.width * factor)
        h = int(image.height * factor)
        self.reset_scroll()

        img = ImageTk.PhotoImage(image.resize((w, h)))
        label = CTkLabelEx(self.main_frame, text="", text_color='white', fg_color="black", width=w, height=h)
        label.configure(image=img)
        label.place(relx=.5, rely=.5, anchor='center')
        label.grid(row=row, column=col, padx=padx, pady=0)

    def clean_master(self):
        try:
            for child in self.main_frame.winfo_children():
                child.destroy()

            self.main_frame.destroy()
            self.main_frame = None
            self.main_frame = customtkinter.CTkScrollableFrame(self, width=WIDTH, height=HEIGHT)
            self.main_frame.grid(row=0, column=0, padx=0, pady=0)
        except:
            pass
        
    def back_in_history(self, event):
        if len(self.history) > 1:
            self.history.pop()
            self.clean_master()
            self.draw()
        if len(self.focused_selection_history) > 0:
            self.focused = self.focused_selection_history.pop()
            self.main_frame.winfo_children()[self.focused].focus()

    def OnSingleClick(self, event):
        self.focused = 0

        metadata = event.widget.master.get_metadata()
        last_in_history = self.history[-1]["type"]
        toast_msg = ""

        if last_in_history == EntryType.SHELF and metadata["id"] == int(EntryType.CLEAN_CACHE):
            if self.kavita.get_offline_mode():
                return
            self.kavita.clear_manga_cache()
            toast_msg = "Cache cleaned!"
        elif last_in_history == EntryType.SHELF and metadata["id"] == int(EntryType.UPDATE_SERVER_LIB):
            if self.kavita.get_offline_mode():
                return
            self.kavita.update_server_library()
            toast_msg = "Kavita library update requested!"
        elif last_in_history == EntryType.SHELF and metadata["id"] == int(EntryType.ENABLE_OFFLINE_MODE):
            self.kavita.offline_mode = not self.kavita.offline_mode
            if self.kavita.offline_mode:
                toast_msg = "Offline mode enabled"
            else:
                toast_msg = "Offline mode disabled"
        elif last_in_history == EntryType.SHELF and metadata["id"] == int(EntryType.EXIT):
            self.destructor()
        elif last_in_history == EntryType.SHELF:
            self.history.append({ "type": EntryType.LIBRARY, "parent_id": -1})
        elif last_in_history == EntryType.LIBRARY:
            self.history.append({ "type": EntryType.SERIE, "parent_id": metadata["id"]})
        elif last_in_history == EntryType.SERIE:
            self.history.append({ "type": EntryType.VOLUME, "parent_id": metadata["id"], "title": metadata["title"]})
        elif last_in_history == EntryType.VOLUME:
            self.history.append({ 
                                 "type": EntryType.PICTURE, 
                                 "parent_id": metadata["id"],
                                 "chapter_id": metadata["chapter_id"],
                                 "read": metadata["read"], 
                                 "pages": metadata["pages"]
                                 })
        self.update()

        idx = 10
        while idx > 0:
            self.scroll_up(None)
            idx -= 1
        
        if len(toast_msg):
            self.draw_toast(toast_msg)
        

    def OnFocusIn(self, event):
        event.widget.master.configure(text_color="red")

    def OnFocusOut(self, event):
        event.widget.master.configure(text_color="white")
        
    def is_small_pic(self, filepath):
        image = Image.open(filepath)
        return (image.width / image.height) < 1.0

    def set_focus_on(self):
        count = len(self.main_frame.winfo_children())
        if count > 0 and self.focused < count:
            self.main_frame.winfo_children()[self.focused].focus()

    def previous_page(self, event):
        last_in_history = self.history[-1]["type"]
        
        if last_in_history == EntryType.PICTURE:
            self.clean_master()
            chapter_id = self.history[-1]["chapter_id"]
            current_page = self.history[-1]["read"]
            if current_page > 0:
                self.history[-1]["read"] -= 1
                filepath = self.kavita.get_picture(chapter_id, self.history[-1]["read"])
                self.draw_pic(filepath)
        else:
            if self.focused != 0:
                self.focused -= 1
            self.set_focus_on()
    
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
                self.draw_pic(filepath)
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
            if self.focused + 1 < count:
                self.focused += 1
            self.main_frame.winfo_children()[self.focused].focus()

    def enter_to(self, event):
        self.focused_selection_history.append(self.focused)
        self.main_frame.focus_get().event_generate("<Button-1>")
        
    def set_volume_as_read(self, event):
        metadata = event.widget.master.get_metadata()
        last_in_history = self.history[-1]
        
        if last_in_history["type"] == EntryType.VOLUME:
            seriesId = last_in_history["parent_id"]
            volume = metadata["id"]
            self.kavita.set_volume_as_read(seriesId, volume)
            self.update()
            self.draw_toast(f"Volume {volume} marked as read")

if __name__ == "__main__":
    app = App()
    app.mainloop()
import tkinter
from tkinter import ttk
import tkinter.messagebox
import customtkinter
from PIL import Image, ImageTk
from kavita import * 
from enum import Enum
import io

customtkinter.set_appearance_mode("Dark")  # Modes: "System" (standard), "Dark", "Light"
customtkinter.set_default_color_theme("dark-blue")  # Themes: "blue" (standard), "green", "dark-blue"

class DisplayType(Enum):
    TWOPICSSIDEBYSIDE = 1
    SINGLE = 2

class EntryType(Enum):
    SHELF = 1
    LIBRARY = 2
    SERIE = 3
    VOLUME = 4
    PICTURE = 5

URL = "http://192.168.5.49:5001/api/"
THUMB_PATH = "./assets/thumbnail.png"
CACHE_PATH = "./cache"

username = "boddicheg"
password = "dyd6ZNU.aby*mqd6fwd"
api_key = "8df0fde8-8229-464c-ae0c-fd58a1a35b11"

DISPLAY_TYPE = DisplayType.SINGLE

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
        
        self.kavita = KavitaAPI(URL, username, password, api_key)
        self.history = [{ "type": EntryType.SHELF, "parent_id": -1}]
        self.draw()

        self.bind("<BackSpace>", self.back_in_history)
        self.bind("<Left>", self.previous_page)
        self.bind("<Right>", self.next_page)
        self.bind("<Down>", self.scroll_down)
        self.bind("<Up>", self.scroll_up)
        self.bind("<Return>", self.enter_to)
        self.bind("<y>", self.clean_cache)
        
        self.focused = 0
        
    def clean_cache(self, event):
        self.kavita.clear_manga_cache()
    
    def scroll_down(self, event):
        self.main_frame._parent_canvas.yview_scroll(1, "units")
        
    def scroll_up(self, event):
        self.main_frame._parent_canvas.yview_scroll(-1, "units")
        
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
        if state == EntryType.PICTURE:            
            self.history[-1]["read"] -= 1
            self.next_page(None)
        else:
            for i, entry in enumerate(entries, start=0):
                col = int(i % 8)
                row = int(i / 8)
                
                # add image and title
                title = entry["title"][0:14] + "..." if len(entry["title"]) > 13 else entry["title"]
                completed = False
                
                if state == EntryType.SERIE:
                    filepath = self.kavita.get_serie_cover(entry["id"])
                    img = ImageTk.PhotoImage(Image.open(filepath).resize((150, 200)))
                    completed = entry["read"] == 100
                    print(entry["read"])
                    title += f"\nRead: {entry['read']:.1f}%" 
                elif state == EntryType.VOLUME:
                    filepath = self.kavita.get_volume_cover(entry["id"])
                    img = ImageTk.PhotoImage(Image.open(filepath).resize((150, 200))) 
                    title = entry["title"]
                    completed = entry["pages"] == entry["read"]
                elif state == EntryType.SHELF:
                    cache = cache_size()
                    title += f"\nCache: {cache:.1f}Gb"
                    img = ImageTk.PhotoImage(Image.open(THUMB_PATH).resize((150, 200)))
                else:
                    img = ImageTk.PhotoImage(Image.open(THUMB_PATH).resize((150, 200)))
                
                label = CTkLabelEx(self.main_frame, 
                                   text=title, 
                                   text_color='white',
                                   fg_color="black" if not completed else "green", 
                                   width=150, 
                                   height=200,
                                   compound="bottom")
                label.configure(image=img)
                label.set_metadata(entry)
                label.bind("<Button-1>", self.OnSingleClick)
                label.bind("<FocusIn>", self.OnFocusIn)
                label.bind("<FocusOut>", self.OnFocusOut)
                label.grid(row=row, column=col, padx=5, pady=5)
                
    def draw_pic(self, filepath, left = False, first_small = False):
        image = Image.open(filepath)
        if DISPLAY_TYPE == DisplayType.TWOPICSSIDEBYSIDE:
            w = PIC_WIDTH if first_small else BIG_PIC_WIDTH
            h = PIC_HEIGHT if first_small else BIG_PIC_HEIGHT
            row = 0
            col = 0 if left else 1
            if first_small:
                padx = (120, 30) if left else (0, 0)
            else:
                padx = (170, 0)

        elif DISPLAY_TYPE == DisplayType.SINGLE:
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
        
        if last_in_history == EntryType.SHELF:
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
                if DISPLAY_TYPE == DisplayType.TWOPICSSIDEBYSIDE:
                    print(f"Current position: {chapter_id}, page {current_page - 3}")
                    # Draw two pics: 
                    self.history[-1]["read"] -= 3
                    filepath = self.kavita.get_picture(chapter_id, self.history[-1]["read"])
                    draw_both = self.is_small_pic(filepath)
                    # check w/h result and decide should we draw one or two(first one can be big)
                    self.draw_pic(filepath, True, draw_both)
                    if draw_both:
                        self.history[-1]["read"] += 1
                        filepath = self.kavita.get_picture(chapter_id, self.history[-1]["read"])
                        self.draw_pic(filepath, False, draw_both)
                elif DISPLAY_TYPE == DisplayType.SINGLE:
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
                if DISPLAY_TYPE == DisplayType.TWOPICSSIDEBYSIDE:
                    
                    # Draw two pics: 
                    self.history[-1]["read"] += 1
                    filepath = self.kavita.get_picture(chapter_id, self.history[-1]["read"])
                    is_first_small = self.is_small_pic(filepath)
                    # check w/h result and decide should we draw one or two(first one can be big)
                    self.draw_pic(filepath, True, is_first_small)
                    if is_first_small:
                        filepath = self.kavita.get_picture(chapter_id, self.history[-1]["read"] + 1)
                        is_second_small = self.is_small_pic(filepath)
                        if is_second_small:
                            self.history[-1]["read"] += 1
                            self.draw_pic(filepath, False, is_second_small)
                elif DISPLAY_TYPE == DisplayType.SINGLE:
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
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

URL = "http://192.168.5.49:5001/api/"
THUMB_PATH = "./assets/thumbnail.png"
CACHE_PATH = "./cache"

WIDTH = 1280
HEIGHT = 720

PIC_WIDTH = 496
PIC_HEIGHT = 720

BIG_PIC_WIDTH = 960
BIG_PIC_HEIGHT = 720

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
        self.bind("<Key>", self.key_handler)
        self.bind("<Left>", self.previous_page)
        self.bind("<Right>", self.next_page)
        
    def key_handler(self, event):
        print(event.char, event.keysym, event.keycode)
        
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
                
                if state == EntryType.SERIE:
                    filepath = self.kavita.get_serie_cover(entry["id"])
                    img = ImageTk.PhotoImage(Image.open(filepath).resize((150, 200))) 
                elif state == EntryType.VOLUME:
                    filepath = self.kavita.get_volume_cover(entry["id"])
                    img = ImageTk.PhotoImage(Image.open(filepath).resize((150, 200))) 
                    title = entry["title"]
                else:
                    img = ImageTk.PhotoImage(Image.open(THUMB_PATH).resize((150, 200)))
                
                label = CTkLabelEx(self.main_frame, 
                                   text=title, 
                                   text_color='white',
                                   fg_color="black", 
                                   width=150, 
                                   height=200,
                                   compound="bottom")
                label.configure(image=img)
                label.set_metadata(entry)
                # label.place(x=20, y=20)
                label.bind("<Double-1>", self.OnDoubleClick)
                label.grid(row=row, column=col, padx=5, pady=5)
                
    def draw_pic(self, filepath, left = False, first_small = False):
        w = PIC_WIDTH if first_small else BIG_PIC_WIDTH
        h = PIC_HEIGHT if first_small else BIG_PIC_HEIGHT
        row = 0
        col = 0 if left else 1
        if first_small:
            padx = (120, 30) if left else (0, 0)
        else:
            padx = (170, 0)
        img = ImageTk.PhotoImage(Image.open(filepath).resize((w, h)))
        label = CTkLabelEx(self.main_frame, text="", text_color='white', fg_color="black", width=w, height=h)
        label.configure(image=img)
        label.place(relx=.5, rely=.5, anchor='center')
        label.grid(row=row, column=col, padx=padx, pady=5)
            
    def clean_master(self):
        for child in self.main_frame.winfo_children():
            child.destroy()
        
    def back_in_history(self, event):
        if len(self.history) > 1:
            self.history.pop()
            self.clean_master()
            self.draw()
        
    def OnDoubleClick(self, event):
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
        
    def is_small_pic(self, filepath):
        image = Image.open(filepath)
        return (image.width / image.height) < 1.0

    def previous_page(self, event):
        self.clean_master()
        last_in_history = self.history[-1]["type"]
        
        if last_in_history == EntryType.PICTURE:
            chapter_id = self.history[-1]["chapter_id"]
            current_page = self.history[-1]["read"]
            if current_page > 0:
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
    
    def next_page(self, event):
        self.clean_master()
        last_in_history = self.history[-1]["type"]
        
        if last_in_history == EntryType.PICTURE:
            chapter_id = self.history[-1]["chapter_id"]
            current_page = self.history[-1]["read"]
            if current_page <= self.history[-1]["pages"]:
                print(f"Current position: {chapter_id}, page {current_page + 1}")
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
        
if __name__ == "__main__":
    app = App()
    app.mainloop()
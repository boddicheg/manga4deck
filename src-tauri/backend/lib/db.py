from sqlalchemy import create_engine, Column, Integer, String, ForeignKey
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import sessionmaker, relationship

import atexit

# Create a base class for declarative class definitions
Base = declarative_base()

# -----------------------------------------------------------------------------
# Tables
class ServerSettings(Base):
    __tablename__ = 'server_settings'
    id = Column(Integer, primary_key=True)
    key = Column(String, nullable=False, unique=True)
    value = Column(String, nullable=False)

class Library(Base):
    __tablename__ = 'library'
    id = Column(Integer, primary_key=True)
    library_id = Column(Integer, nullable=False)
    title = Column(String, nullable=False)
    
class Series(Base):
    __tablename__ = 'series'
    id = Column(Integer, primary_key=True)
    series_id = Column(Integer, nullable=False)
    title = Column(String, nullable=False)
    read = Column(Integer, nullable=False)
    pages = Column(Integer, nullable=False)

class Volumes(Base):
    __tablename__ = 'volumes'
    id = Column(Integer, primary_key=True)
    volume_id = Column(Integer, nullable=False)
    series_id = Column(Integer, nullable=False)
    chapter_id = Column(Integer, nullable=False)
    title = Column(String, nullable=False)
    read = Column(Integer, nullable=False)
    pages = Column(Integer, nullable=False)
    
class MangaPictures(Base):
    __tablename__ = 'manga_pictures'
    id = Column(Integer, primary_key=True)
    chapter_id = Column(Integer, nullable=False)
    page = Column(Integer, nullable=False)
    filepath = Column(String, nullable=False)

class SerieCovers(Base):
    __tablename__ = 'serie_covers'
    id = Column(Integer, primary_key=True)
    series_id = Column(Integer, nullable=False)
    filepath = Column(String, nullable=False)
    
class VolumeCovers(Base):
    __tablename__ = 'volume_covers'
    id = Column(Integer, primary_key=True)
    volume_id = Column(Integer, nullable=False)
    filepath = Column(String, nullable=False)

class ReadProgress(Base):
    __tablename__ = 'read_progress'
    id = Column(Integer, primary_key=True)
    library_id = Column(Integer, nullable=False)
    series_id = Column(Integer, nullable=False)
    volume_id = Column(Integer, nullable=False)
    chapter_id = Column(Integer, nullable=False)
    page = Column(Integer, nullable=False)
    
g_tables = [ServerSettings, Library, Series, Volumes, MangaPictures, SerieCovers, VolumeCovers, ReadProgress]
# -----------------------------------------------------------------------------

class DBSession:
    def __init__(self, db_path) -> None:
        self.engine = create_engine(f'sqlite:///{db_path}')
        Base.metadata.create_all(self.engine)
        Session = sessionmaker(bind=self.engine)
        self.session = Session()
        
        atexit.register(self.destuctor)
        
    def destuctor(self):
        self.session.close()

# -----------------------------------------------------------------------------
# Server Settings methods
    def set_server_setting(self, key, value):
        """Set a server setting by key"""
        setting = self.session.query(ServerSettings).filter_by(key=key).first()
        if setting:
            setting.value = value
        else:
            self.session.add(ServerSettings(key=key, value=value))
        self.commit_changes()
    
    def get_server_setting(self, key, default=None):
        """Get a server setting by key, return default if not found"""
        setting = self.session.query(ServerSettings).filter_by(key=key).first()
        return setting.value if setting else default
    
    def get_all_server_settings(self):
        """Get all server settings as a dictionary"""
        settings = self.session.query(ServerSettings).all()
        return {setting.key: setting.value for setting in settings}

# -----------------------------------------------------------------------------
# Library methods
    def add_library(self, data):
        keys = ["id", "title"]
        for k in keys:
            if k not in data.keys():
                print(f"-> Can't find key {k} in params")
        count = self.session.query(Library).filter_by(library_id=data["id"]).count()
        if count == 0:
            self.session.add(Library(library_id=data["id"], title=data["title"]))

    def get_libraries(self):
        libraries = self.session.query(Library).all()

        result = []
        for library in libraries:
            result.append({
                "id": library.library_id,
                "title": library.title
            })
        return result

# -----------------------------------------------------------------------------
# Series methods
    def add_series(self, data):
        keys = ["id", "title", "read", "pages"]
        for k in keys:
            if k not in data.keys():
                print(f"-> Can't find key {k} in params")
        count = self.session.query(Series).filter_by(series_id=data["id"]).count()
        if count == 0:
            self.session.add(Series(series_id=data["id"], 
                                    title=data["title"],
                                    read=data["read"],
                                    pages=data["pages"]))
            self.commit_changes()

    def get_series(self):
        # TODO: add library id to save and search
        series = self.session.query(Series).all()
        result = []
        for item in series:
            result.append({
                "id": item.series_id,
                "title": item.title,
                "read": item.read * 100 / item.pages,
                "pages": item.pages
            })

        return result
    
    def is_series_cached(self, id):
        count = self.session.query(Series).filter_by(series_id=id).count()
        return count > 0
    
    def set_series_read_pages(self, series_id, page):
        series = self.session.query(Series).filter_by(series_id=series_id).first()
        if series:
            series.read = page
            self.commit_changes()

# -----------------------------------------------------------------------------
# Volumes methods
    def add_volumes(self, data):
        keys = ["series_id", "volume_id", "chapter_id", "title", "read", "pages"]
        for k in keys:
            if k not in data.keys():
                print(f"-> Can't find key {k} in params")
                
        count = self.session.query(Volumes).filter_by(volume_id=data["volume_id"]).count()
        if count == 0:
            self.session.add(Volumes(series_id=data["series_id"],
                                    volume_id=data["volume_id"],
                                    chapter_id=data["chapter_id"],
                                    title=data["title"],
                                    read=data["read"],
                                    pages=data["pages"]))
            self.commit_changes()

    def get_volumes(self, series_id):
        series = self.session.query(Volumes).filter_by(series_id=series_id).all()
        result = []
        for item in series:
            result.append({
                "volume_id": item.volume_id,
                "series_id": item.series_id,
                "chapter_id": item.chapter_id,
                "title": item.title,
                "read": item.read,
                "pages": item.pages
            })

        return result
    
    def is_volume_cached(self, id):
        count = self.session.query(Volumes).filter_by(volume_id=id).count()
        return count > 0
    
    def set_volume_as_read(self, id, series_id, page=None):
        volume = self.session.query(Volumes).filter_by(volume_id=id, series_id=series_id).first()
        if volume:
            volume.read = page if page else volume.pages
            self.commit_changes()

# -----------------------------------------------------------------------------
# Serie covers methods
    def add_series_cover(self, data):
        keys = ["seriesId", "file"]
        for k in keys:
            if k not in data.keys():
                print(f"-> Can't find key {k} in params")

        self.session.add(SerieCovers(series_id=data["seriesId"], filepath=data["file"]))
    
    def search_series_cover(self, id):
        result = self.session.query(SerieCovers).filter_by(series_id=id).first()
        return result.filepath if result else ""

# -----------------------------------------------------------------------------# 
# Volume covers methods
    def add_volume_cover(self, data):
        keys = ["volumeId", "file"]
        for k in keys:
            if k not in data.keys():
                print(f"-> Can't find key {k} in params")
        self.session.add(VolumeCovers(volume_id=data["volumeId"], filepath=data["file"]))
    
    def search_volume_cover(self, id):
        result = self.session.query(VolumeCovers).filter_by(volume_id=id).first()
        return result.filepath if result else ""

# -----------------------------------------------------------------------------
# Manga pictures methods
    def add_manga_pic(self, data):
        keys = ["chapter_id", "page", "file"]
        for k in keys:
            if k not in data.keys():
                print(f"-> Can't find key {k} in params")
        count = self.session.query(MangaPictures).filter_by(chapter_id=data["chapter_id"], page=data["page"]).count()
        if count == 0:
            self.session.add(MangaPictures(chapter_id=data["chapter_id"], page=data["page"], filepath=data["file"]))
            self.commit_changes()
    
    def search_manga_pics(self, id, page):
        result = self.session.query(MangaPictures).filter_by(chapter_id=id, page=page).first()
        return result.filepath if result else ""

# -----------------------------------------------------------------------------
# Read Progress methods
    def add_progress(self, data):
        keys = [ "series_id", "volume_id", "chapter_id", "page"]
        for k in keys:
            if k not in data.keys():
                print(f"-> Can't find key {k} in params")
                
        count = self.session.query(ReadProgress).filter_by(
            library_id=2, # fix it later data["library_id"],
            series_id=data["series_id"],
            volume_id=data["volume_id"],
            chapter_id=data["chapter_id"],
            page=data["page"]
        ).count()

        if count == 0:
            self.session.add(ReadProgress(library_id=2, # fix it laterdata["library_id"],
                                            series_id=data["series_id"],
                                            volume_id=data["volume_id"],
                                            chapter_id=data["chapter_id"],
                                            page=data["page"]))
            self.commit_changes()
            
    def get_progress(self):
        # TODO: add library id to save and search
        progress = self.session.query(ReadProgress).all()
        result = []
        for item in progress:
            result.append({
                "library_id": item.library_id,
                "series_id": item.series_id,
                "volume_id": item.volume_id,
                "chapter_id": item.chapter_id,
                "page": item.page,
            })

        return result
    
    def clean_progress(self):
        self.session.query(ReadProgress).delete()
        self.commit_changes()
# -----------------------------------------------------------------------------

    def commit_changes(self):
        self.session.commit()
        
    def print(self):
        for c in g_tables:
            rows = self.session.query(c).all()
            print(c.__tablename__)
            for row in rows:
                print("-> ", row.__dict__)
                
    def clean(self):
        for t in g_tables:
            self.session.query(t).delete()
        self.commit_changes()
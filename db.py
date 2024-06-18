from sqlalchemy import create_engine, Column, Integer, String, ForeignKey
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import sessionmaker, relationship

import atexit

# Create a base class for declarative class definitions
Base = declarative_base()

class Library(Base):
    __tablename__ = 'library'
    id = Column(Integer, primary_key=True)
    library_id = Column(Integer, nullable=False)
    title = Column(String, nullable=False)
    
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
# Library methods
    def add_library(self, data):
        keys = ["id", "title"]
        for k in keys:
            if k not in data.keys():
                print(f"-> Can't find key {k} in params")
                
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
    
    def clean_libraries(self):
        self.session.query(Library).delete()
        self.commit_changes()
# -----------------------------------------------------------------------------
    def commit_changes(self):
        self.session.commit()
        
    def print(self):
        classes = [Library]
        for c in classes:
            rows = self.session.query(c).all()
            print(c.__tablename__)
            for row in rows:
                print("-> ", row.__dict__)
import PyInstaller.__main__
import shutil

# https://www.pythonkitchen.com/packaging-an-sqlite-db-included-crud-pyqt5-app-using-pyinstaller/

PyInstaller.__main__.run([
    'app.py',
    '--onefile',
    "--console",
    "--name=app.exe"
], )

# shutil.copytree('path/', 'dist/path')
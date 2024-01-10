python -m ensurepip --upgrade
echo 'PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
rm -rf /home/deck/manga4deck
git clone https://github.com/boddicheg/manga4deck.git /home/deck/manga4deck
cd /home/deck/manga4deck
pip install -r req.txt
chmod +x run.sh
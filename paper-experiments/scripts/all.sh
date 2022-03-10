echo '### Server FPS'
python3 fps.py $1/server.csv
echo '### Client FPS'
python3 fps.py $1/client.csv

echo '### Server analytics'
python3 analytics.py $1/server.csv
echo '### Client analytics'
python3 analytics.py $1/client.csv

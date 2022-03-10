import pandas
import sys

df = pandas.read_csv(sys.argv[1])
df['capture_datetime'] = pandas.to_datetime(df['capture_timestamp'], unit='ms')
df['capture_datetime'] = df['capture_datetime'].astype('datetime64[s]')

df = df.set_index('capture_datetime').drop('capture_timestamp', axis=1)
df = df.groupby(['capture_datetime']).size()

pandas.options.display.float_format = '{:,.2f}'.format
print(df.mean())

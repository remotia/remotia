import pandas
import sys

df = pandas.read_csv(sys.argv[1])
df['capture_datetime'] = pandas.to_datetime(df['capture_timestamp'], unit='ms')

df = df.set_index('capture_datetime').drop('capture_timestamp', axis=1)

df = df.resample('s').mean()

df = df.mean()

pandas.options.display.float_format = '{:,.2f}'.format
print(df)

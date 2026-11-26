import struct,sys
from pathlib import Path
d=Path(sys.argv[1]); out=Path(sys.argv[2])
sizes=[16,20,24,32,40,48,64,256]
data=[(s,(d/f"{s}.png").read_bytes()) for s in sizes]
hdr=struct.pack("<HHH",0,1,len(data)); off=6+16*len(data); ent=b""; body=b""
for s,b in data:
    ent+=struct.pack("<BBBBHHII",s%256,s%256,0,0,1,32,len(b),off+len(body)); body+=b
out.write_bytes(hdr+ent+body)

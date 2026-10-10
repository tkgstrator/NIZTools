import hashlib, re, struct
from pathlib import Path
path=Path('/private/tmp/niz-analysis-20261010/Software/66EC(XRGB)BleHWI.dll')
b=path.read_bytes()
print('Path:',path, 'Bytes:', len(b),'SHA256:',hashlib.sha256(b).hexdigest())
p=struct.unpack_from('<I',b,0x3c)[0]
n=struct.unpack_from('<H',b,p+6)[0]
o=p+24
os=struct.unpack_from('<H',b,p+20)[0]
ss=[]
for i in range(n):
    q=o+os+i*40
    name=b[q:q+8].split(b'\0')[0].decode()
    vs,va,rs,ro=struct.unpack_from('<IIII',b,q+8)
    ss.append((name,vs,va,rs,ro))
    print('SECTION',name,'virtual_size',hex(vs),'RVA',hex(va),'raw_size',hex(rs),'raw_offset',hex(ro))
def off(rva):
    for name,vs,va,rs,ro in ss:
        if va<=rva<va+rs:
            return ro+rva-va
    raise ValueError(hex(rva))
def cstr(rva):
    q=off(rva)
    return b[q:b.index(b'\0',q)].decode()
r=struct.unpack_from('<I',b,o+104)[0]
while True:
    il,t,fw,dll_name,ia=struct.unpack_from('<IIIII',b,off(r))
    if not il:
        break
    name=cstr(dll_name)
    idx=0
    while True:
        v=struct.unpack_from('<I',b,off(il+idx*4))[0]
        if not v:
            break
        print('IAT',hex(0x10000000+ia+idx*4),name,cstr(v+2))
        idx+=1
    r+=20
for a in [0xd3ec,0xd3f8,0xd410,0xd41c,0xd428,0xd434,0xd440]:
    q=off(a)
    raw=b[q:q+64]
    end=next(i for i in range(0,len(raw),2) if raw[i:i+2]==b'\0\0')
    print('UTF16',hex(0x10000000+a),'raw',hex(q),repr(raw[:end].decode('utf-16-le')))
for a in [0xd47c,0xd498,0xd098,0xd4a4,0xd4b0]:
    print('VTABLE',hex(0x10000000+a),'raw',hex(off(a)),[hex(x) for x in struct.unpack_from('<III',b,off(a))])

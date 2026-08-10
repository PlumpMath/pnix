#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/climate/nasa-firms-api-catalog"
OUT="$ROOT/stdlib/lib/corpus/nasa-firms-api-catalog.generated.px"
python3 - "$SRC" "$OUT" <<'PY'
import pathlib, sys, json, hashlib, re, html, urllib.parse
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('$','DOLLAR_SIGN').replace('\n',' ').replace('\r','')+'"'
def lit(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(lit(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {lit(val)};' for k,val in v.items()) + ' }'
    return esc('' if v is None else v)
def clean(s): return re.sub(r'\s+',' ',html.unescape(str(s or ''))).strip()
def sig(txt):
    b=txt.encode() if isinstance(txt,str) else txt
    return {'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
def page_meta(path,page_id,url):
    txt=path.read_text(errors='ignore')
    mt=re.search(r'<title[^>]*>(.*?)</title>',txt,re.I|re.S)
    desc=''
    md=re.search(r'<meta\s+[^>]*name=["\']description["\'][^>]*content=["\']([^"\']+)["\']',txt,re.I|re.S)
    if md: desc=clean(md.group(1))[:260]
    links=[]
    for m in re.finditer(r'<a\s+[^>]*href=["\']([^"\']+)["\'][^>]*>(.*?)</a>', txt, re.I|re.S):
        href=html.unescape(m.group(1)); label=clean(re.sub('<.*?>',' ',m.group(2)))[:180]
        if href and not href.startswith('#'):
            low=(href+' '+label).lower()
            if any(k in low for k in ['api','firms','lance','earthdata','fire','map_key','modis','viirs','data_availability','missing_data']):
                links.append({'kind':'doc_or_api_ref_payload_excluded','label':label,'href':urllib.parse.urljoin(url,href)})
    tokens=sorted(set([x.lower() for x in re.findall(r'\b(?:FIRMS|LANCE|API|area|country|countries|data_availability|missing_data|map_key|MODIS|VIIRS|NOAA20|SNPP|NRT|RT|URT|CSV|GeoJSON|KML)\b',txt,re.I)]))
    return {'page':page_id,'url':url,'file':path.name,'title':clean(re.sub('<.*?>',' ',mt.group(1)))[:220] if mt else '', 'description':desc, 'html_sig':sig(txt),'tokens':tokens,'links':links[:160]}
manifest=json.loads((src/'source-manifest.json').read_text()) if (src/'source-manifest.json').exists() else {}
page_specs=[
 ('api.html','api','https://firms.modaps.eosdis.nasa.gov/api/'),
 ('area.html','area','https://firms.modaps.eosdis.nasa.gov/api/area/'),
 ('data_availability.html','data_availability','https://firms.modaps.eosdis.nasa.gov/api/data_availability/'),
 ('kml_fire_footprints.html','kml_fire_footprints','https://firms.modaps.eosdis.nasa.gov/api/kml_fire_footprints/'),
 ('missing_data.html','missing_data','https://firms.modaps.eosdis.nasa.gov/api/missing_data/'),
 ('map_key.html','map_key','https://firms.modaps.eosdis.nasa.gov/api/map_key/'),
 ('earthdata_tool.html','earthdata_tool','https://www.earthdata.nasa.gov/data/tools/firms')
]
pages=[]
for name,pid,url in page_specs:
    p=src/name
    if p.exists(): pages.append(page_meta(p,pid,url))
endpoints=[
 {'name':'area','path':'/api/area/{format}/{map_key}/{source}/{area_coordinates}/{day_range}/{date}','formats':['csv','json','geojson'],'requires_map_key':True,'payload_policy':'not called by ingest; active fire/hotspot records excluded'},
 {'name':'countries','path':'/api/countries','formats':['csv'], 'requires_map_key':False,'payload_policy':'not called by ingest; country list payload excluded'},
 {'name':'country','path':'/api/country/{format}/{map_key}/{source}/{country_code}/{day_range}/{date}','formats':['csv','json','geojson'],'requires_map_key':True,'payload_policy':'not called by ingest; active fire/hotspot records excluded'},
 {'name':'data_availability','path':'/api/data_availability/{map_key}/{source}','formats':['csv','json'],'requires_map_key':True,'payload_policy':'not called by ingest; availability payload excluded'},
 {'name':'kml_fire_footprints','path':'/api/kml_fire_footprints/{map_key}/{source}/{area_coordinates}/{day_range}/{date}','formats':['kml'],'requires_map_key':True,'payload_policy':'not called by ingest; KML fire footprint payload excluded'},
 {'name':'missing_data','path':'/api/missing_data/{map_key}/{source}','formats':['csv','json'],'requires_map_key':True,'payload_policy':'not called by ingest; missing-date payload excluded'},
 {'name':'map_key','path':'/api/map_key','formats':['html'], 'requires_map_key':False,'payload_policy':'MAP_KEY credential request/status page only; credentials excluded'}
]
sources=['MODIS_NRT','MODIS_SP','VIIRS_SNPP_NRT','VIIRS_SNPP_SP','VIIRS_NOAA20_NRT','VIIRS_NOAA20_SP','LANDSAT_NRT']
files=[]
for p in sorted(src.iterdir()):
    if p.is_file(): files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
obj={'schema':'climate.nasa_firms.api_catalog.v1','source':'NASA FIRMS API documentation/catalog metadata','license':'NASA public documentation/catalog metadata; FIRMS fire/hotspot payloads excluded','summary':{'pages':len(pages),'links':sum(len(x['links']) for x in pages),'endpoints':len(endpoints),'source_tokens':len(sources),'files':len(files)},'policy':'NASA FIRMS public API documentation/catalog page metadata and endpoint tokens only; active fire/hotspot records, satellite observation values, CSV/GeoJSON/KML payloads, map tiles, MAP_KEY credentials, forecasts/life-safety decisions, evacuation/emergency guidance, and graph wiring excluded','manifest':manifest,'files':files,'pages':pages,'endpoints':endpoints,'source_tokens':sources}
out.write_text('# GENERATED by scripts/gen-climate-nasa-firms-api-catalog.sh. Do not edit. Gitignored.\n# Source: NASA FIRMS API catalog metadata only; data payloads/MAP_KEY/life-safety decisions excluded.\n'+lit(obj)+'\n',encoding='utf-8')
print(f'generated {out}: pages={len(pages)} endpoints={len(endpoints)} bytes={out.stat().st_size}')
PY

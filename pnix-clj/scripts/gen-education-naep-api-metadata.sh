#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/education/naep-api-metadata"
OUT="$ROOT/stdlib/lib/corpus/naep-api-metadata.generated.px"
LIMIT="${NAEP_INDEPENDENT_VARIABLE_LIMIT:-80}"
python3 - "$SRC" "$OUT" "$LIMIT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); limit=int(sys.argv[3])

def esc(s):
    return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r','')+'"'
def lit(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(lit(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {lit(val)};' for k,val in v.items()) + ' }'
    return esc('' if v is None else v)

request_types=[
  {'code':'data','kind':'result'},
  {'code':'sigacrossyear','kind':'significance'},
  {'code':'sigacrossjuris','kind':'significance'},
  {'code':'sigacrossvalue','kind':'significance'},
  {'code':'gaponyearacrossjuris','kind':'gap'},
  {'code':'gaponvaracrossyear','kind':'gap'},
  {'code':'gaponvaracrossjuris','kind':'gap'},
  {'code':'gaponvarandyearacrossjuris','kind':'gap'},
  {'code':'category','kind':'metadata'},
  {'code':'independentvariables','kind':'metadata'}
]
parameters=[
  {'name':'Type','role':'request_type'}, {'name':'Subject','role':'assessment_subject'},
  {'name':'Grade','role':'grade'}, {'name':'Cohort','role':'age_or_grade_cohort'},
  {'name':'Subscale','role':'assessment_subscale'}, {'name':'Variable','role':'independent_variable_code'},
  {'name':'ComparisonValues','role':'comparison_category_values'}, {'name':'CategoryIndex','role':'category_index'},
  {'name':'Jurisdiction','role':'jurisdiction_code'}, {'name':'StatType','role':'statistic_type_code'},
  {'name':'Year','role':'assessment_year_or_relative_token'}, {'name':'StackType','role':'cross_tab_layout'}
]
subjects=[
  {'code':'mathematics','alias':'MAT'}, {'code':'reading','alias':'RED'}, {'code':'science','alias':'SCI'},
  {'code':'writing','alias':'WRI'}, {'code':'civics','alias':'CIV'}, {'code':'history','alias':'USH'},
  {'code':'geography','alias':'GEO'}, {'code':'economics','alias':'ECO'}
]
grades=['4','8','12']
cohorts=[{'code':'1','label':'grade4_or_age9'},{'code':'2','label':'grade8_or_age13'},{'code':'3','label':'grade12_or_age17'}]
year_tokens=['R2','R3','Base','Current','Prior']
stat_types=[
  {'code':'MN:MN','family':'mean'}, {'code':'RP:RP','family':'percentage'},
  {'code':'ALD:BA','family':'achievement_level'}, {'code':'ALD:PR','family':'achievement_level'},
  {'code':'ALD:AD','family':'achievement_level'}, {'code':'ALC:BB','family':'achievement_level'},
  {'code':'ALC:AB','family':'achievement_level'}, {'code':'ALC:AP','family':'achievement_level'},
  {'code':'ALC:AD','family':'achievement_level'}, {'code':'SD:SD','family':'standard_deviation'},
  {'code':'PC:P1','family':'percentile'}, {'code':'PC:P2','family':'percentile'},
  {'code':'PC:P5','family':'percentile'}, {'code':'PC:P7','family':'percentile'}, {'code':'PC:P9','family':'percentile'}
]
common_variables=[
  {'code':'TOTAL','kind':'total'}, {'code':'SDRACE','kind':'student_group'}, {'code':'SRACE10','kind':'student_group'},
  {'code':'GENDER','kind':'student_group'}, {'code':'SLUNCH3','kind':'student_group'}, {'code':'PARED','kind':'student_group'},
  {'code':'SCHTYPE','kind':'school'}, {'code':'CHRTRPT','kind':'school'}, {'code':'UTOL4','kind':'location'},
  {'code':'CENSREG','kind':'location'}, {'code':'IEP','kind':'program'}, {'code':'LEP','kind':'program'}
]
ind=[]
p=src/'independentvariables-red8.json'
if p.exists():
    try:
        data=json.loads(p.read_text(encoding='utf-8'))
        res=data.get('result', []) if isinstance(data,dict) else []
        rows=[]
        if isinstance(res,dict):
            for v in res.values():
                if isinstance(v,list): rows.extend({'_ctx':{},'_var':x} for x in v)
                elif isinstance(v,dict): rows.append({'_ctx':{},'_var':v})
        elif isinstance(res,list):
            for item in res:
                if isinstance(item,dict) and isinstance(item.get('variables'),list):
                    ctx={k:item.get(k) for k in ['subject','cohort','year','sample']}
                    rows.extend({'_ctx':ctx,'_var':x} for x in item.get('variables',[]))
                else:
                    rows.append({'_ctx':{},'_var':item})
        seen=set()
        for box in rows:
            r=box.get('_var') if isinstance(box,dict) else box
            ctx=box.get('_ctx',{}) if isinstance(box,dict) else {}
            if not isinstance(r,dict): continue
            name=r.get('varName') or r.get('name') or r.get('variable') or r.get('value')
            if not name or name in seen: continue
            seen.add(name)
            ind.append({'subject':str(ctx.get('subject') or 'RED'),'cohort':str(ctx.get('cohort') or '2'),'year':str(ctx.get('year') or ''),'sample':str(ctx.get('sample') or ''),'var_name':str(name),'short_label':str(r.get('shortLabel') or r.get('label') or '')[:80]})
            if len(ind)>=limit: break
    except Exception as e:
        ind.append({'subject':'RED','cohort':'2','years':'1998,2019','var_name':'PARSE_ERROR','short_label':str(e)[:80]})

obj={
  'schema':'education.naep_api_metadata.v1',
  'source':'NAEP Data Service API metadata',
  'license':'US-PD',
  'endpoint':'https://www.nationsreportcard.gov/DataService/GetAdhocData.aspx',
  'summary':{'request_types':len(request_types),'parameters':len(parameters),'subjects':len(subjects),'stat_types':len(stat_types),'independent_variables':len(ind)},
  'policy':'metadata/code vocabulary only; item body, student responses, result values, prose analysis, and graph wiring excluded',
  'request_types':request_types,
  'parameters':parameters,
  'subjects':subjects,
  'grades':grades,
  'cohorts':cohorts,
  'year_tokens':year_tokens,
  'stack_types':['ColThenRow','RowThenCol'],
  'stat_types':stat_types,
  'common_variables':common_variables,
  'independent_variables':ind
}
out.write_text('# GENERATED by scripts/gen-education-naep-api-metadata.sh. Do not edit. Gitignored.\n# Source: NAEP Data Service API docs + bounded independentvariables metadata.\n# Policy: code/API metadata only; item body/student responses/result values/prose analysis excluded.\n'+lit(obj)+'\n',encoding='utf-8')
print(f'generated {out}: request_types={len(request_types)} parameters={len(parameters)} independent_variables={len(ind)} bytes={out.stat().st_size}')
PY

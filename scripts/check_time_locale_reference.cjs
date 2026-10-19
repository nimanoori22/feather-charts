const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{spawnSync}=require('node:child_process');
const root=path.resolve('../lightweight-charts');
const dependency=name=>require(require.resolve(name,{paths:[root]}));
const hash=file=>createHash('sha256').update(fs.readFileSync(file)).digest('hex');
async function main(){
 const ts = dependency('typescript');
 const bundle = await dependency('rollup').rollup({
  input: 'time-locale-entry',
  plugins: [{name:'entry',resolveId(id){if(id==='time-locale-entry')return id;},load(id){if(id==='time-locale-entry')return `export {defaultTickMarkFormatter} from '${root}/src/model/horz-scale-behavior-time/default-tick-mark-formatter.ts';export {formatDate} from '${root}/src/formatters/format-date.ts';export {TickMarkType} from '${root}/src/model/horz-scale-behavior-time/types.ts';`;}},{ name: 'local-typescript',
   resolveId(id, importer) {
    if (importer && id.startsWith('.')) {
     const candidate = path.resolve(path.dirname(importer), id);
     if (fs.existsSync(candidate + '.ts')) return candidate + '.ts';
    }
   },
   transform(code, id) {
    if (!id.endsWith('.ts')) return null;
    return ts.transpileModule(code, { compilerOptions: {
     target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext,
    }}).outputText.replaceAll('process.env.NODE_ENV', '"development"')
      .replaceAll('process.env.BUILD_VERSION', '"5.2.1-local"')
      .replaceAll('MouseEventButton.Left', '0').replaceAll('MouseEventButton.Middle', '1');
   },
  }, dependency('@rollup/plugin-node-resolve').nodeResolve()],
 });
 // transpileModule does not inline ambient const enums; include their source identity.
 bundle.watchFiles.push(path.join(root, 'src/typings/_global-types/index.d.ts'));
 const files = Object.fromEntries(bundle.watchFiles.filter(p => p.startsWith(root + '/src/')).sort()
  .map(p => [path.relative(root, p), hash(p)]));
 const manifest = 'docs/porting/time-locale-source.json';
 if (process.argv.includes('--freeze')) fs.writeFileSync(manifest, JSON.stringify({ files }, null, 2) + '\n');
 else assert.deepEqual(files, JSON.parse(fs.readFileSync(manifest)).files, 'source identity changed');
 const {output}=await bundle.generate({format:'iife',name:'TimeLocaleSource'});await bundle.close();
 fs.mkdirSync('target/time-locale',{recursive:true});
 const browser=await dependency('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true,args:['--lang=en-US']});
 try {
  const page=await browser.newPage();await page.emulateTimezone('UTC');await page.addScriptTag({content:output[0].code});
  const result=await page.evaluate(()=>{
   const S=TimeLocaleSource;
   const locales=['en-US','fr-FR','de-DE','ar-EG','en','fr','de','ar','default','EN-us','de-DE-u-nu-arab','ar-EG-u-nu-latn','ar-EG-u-nu-arabext','en-US-u-hc-h23','de-DE-u-ca-gregory-nu-latn','de-DE-u-ca-islamic','fa-IR','es-ES','ja-JP','zz-ZZ','not_a_locale','en-US-u-nu-foobar','de-DE-u-nu-arab-ca-gregory'];
   const timestamps=[1704198645,Date.UTC(2024,8,1)/1000,Date.UTC(2023,11,31,23,59,59)/1000,0,-0.5,-0.0001,1.9999999995];
   const safe=fn=>{try{return fn();}catch(e){return {error:e.name};}};
   return locales.map(locale=>({locale,resolved:safe(()=>new Intl.DateTimeFormat(locale).resolvedOptions()),ticks:timestamps.map(timestamp=>({timestamp,values:[0,1,2,3,4].map(kind=>safe(()=>S.defaultTickMarkFormatter({timestamp},kind,locale)))})),months:Array.from({length:12},(_,month)=>safe(()=>S.formatDate(new Date(Date.UTC(2024,month,2)),'yyyy-MM-dd MMM MMMM',locale)))}));
  });
  const fixture='docs/porting/time-locale-states.json';
  if(process.argv.includes('--freeze'))fs.writeFileSync(fixture,JSON.stringify(result,null,2)+'\n');else assert.deepEqual(result,JSON.parse(fs.readFileSync(fixture)),'source locale behavior changed');
  fs.writeFileSync('target/time-locale/source-results.json',JSON.stringify(result,null,2)+'\n');
  fs.writeFileSync('target/time-locale/runtime.json', JSON.stringify({browser:await browser.version(),node:process.version,icu:process.versions.icu,cldr:process.versions.cldr,timezone:'UTC',hostLocale:'en-US'},null,2)+'\n');
  if(!process.argv.includes('--source-only')){
   const native=spawnSync('cargo',['test','--offline','--test','time_locale','--','--nocapture'],{encoding:'utf8',maxBuffer:32*1024*1024});
   fs.writeFileSync('target/time-locale/native.log',native.stdout+native.stderr);
   assert.equal(native.status,0,'native locale tests failed');
   const rows=native.stdout.split('\n').filter(l=>l.startsWith('TIME_LOCALE_REFERENCE ')).map(l=>JSON.parse(l.slice('TIME_LOCALE_REFERENCE '.length)));
   assert.equal(rows.length,16);
   for(const row of rows){const reference=result.find(r=>r.locale===row.locale);assert.deepEqual(row,{locale:reference.locale,ticks:reference.ticks,months:reference.months},row.locale);}
   console.log(`TIME_LOCALE_SOURCE_NATIVE ${rows.length} locales: ${rows.length*7*5} ticks, ${rows.length*12} month/date labels`);
  }
  console.log(JSON.stringify(result.filter(r=>['en-US','fr-FR','de-DE','ar-EG'].includes(r.locale)).map(r=>({locale:r.locale,midnight:r.ticks[1].values,months:r.months})),null,2));
 }finally{await browser.close();}
}
main().catch(e=>{console.error(e);process.exitCode=1;});

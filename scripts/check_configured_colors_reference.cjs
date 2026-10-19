const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {createHash}=require('node:crypto'),{spawnSync}=require('node:child_process');
const root=path.resolve('../lightweight-charts');
const dependency=name=>require(require.resolve(name,{paths:[root]}));
const hash=file=>createHash('sha256').update(fs.readFileSync(file)).digest('hex');
async function main(){
 const ts = dependency('typescript');
 const bundle = await dependency('rollup').rollup({
  input: 'configured-colors-entry',
  plugins: [{name:'entry',resolveId(id){if(id==='configured-colors-entry')return id;},load(id){if(id==='configured-colors-entry')return `export {ColorParser} from '${root}/src/model/colors.ts';`;}},{ name: 'local-typescript',
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
 const manifest = 'docs/porting/configured-colors-source.json';
 if (process.argv.includes('--freeze')) fs.writeFileSync(manifest, JSON.stringify({ files }, null, 2) + '\n');
 else assert.deepEqual(files, JSON.parse(fs.readFileSync(manifest)).files, 'source identity changed');
 const {output}=await bundle.generate({format:'iife',name:'ConfiguredColorsSource'});await bundle.close();
 fs.mkdirSync('target/configured-colors',{recursive:true});
 const browser=await dependency('puppeteer').launch({executablePath:'/usr/bin/chromium',headless:true,args:['--lang=en-US']});
 try {
  const page=await browser.newPage();await page.emulateTimezone('UTC');await page.addScriptTag({content:output[0].code});
  const result=await page.evaluate(()=>{
   const calls=[],dark='color(display-p3 0.2 0.3 0.4 / 0.5)',light='color(display-p3 0.8 0.9 1 / 0.25)',unknown='color(display-p3 0 1 0)';
   const p=new ConfiguredColorsSource.ColorParser([v=>{calls.push('first:'+v);return null;},v=>{calls.push('second:'+v);return v===dark?[20,40,60,0.5]:v===light?[220,230,240,0.25]:null;},v=>{calls.push('third:'+v);return null;}]);
   const result={gray:p.colorStringToGrayscale(dark),contrast:p.generateContrastColors(dark),alpha:p.applyAlpha(dark,0.4),transparent:p.applyAlpha('transparent',0.3),gradient:p.gradientColorAtPercent(dark,light,0.5),light:p.generateContrastColors(light),css:p.generateContrastColors('#fff')};
   p.colorStringToGrayscale(dark);
   let failures=0;for(let i=0;i<2;i++){try{p.colorStringToGrayscale(unknown);}catch{failures++;}}
   const forms=['color(display-p3 1 0 0)','color(srgb 0.2 0.3 0.4)','color(srgb-linear 0.2 0.3 0.4)','lab(50% 0 0)','lch(50% 0 0)','oklab(50% 0 0)','oklch(50% 0 0)','hsl(120 50% 50%)','hwb(120 10% 20%)','#abcdef'].map(token=>{let count=0;const parser=new ConfiguredColorsSource.ColorParser([()=>{count++;return[10,20,30,0.5];}]);return{token,gray:parser.colorStringToGrayscale(token),calls:count};});
   return {...result,failures,calls,forms};
  });
  const fixture='docs/porting/configured-colors-states.json';
  if(process.argv.includes('--freeze'))fs.writeFileSync(fixture,JSON.stringify(result,null,2)+'\n');else assert.deepEqual(result,JSON.parse(fs.readFileSync(fixture)),'source color behavior changed');
  fs.writeFileSync('target/configured-colors/source-results.json',JSON.stringify(result,null,2)+'\n');
  if(!process.argv.includes('--source-only')){
   const native=spawnSync('cargo',['test','--offline','--test','configured_colors','--','--nocapture'],{encoding:'utf8',maxBuffer:32*1024*1024});
   fs.writeFileSync('target/configured-colors/native.log',native.stdout+native.stderr);
   assert.equal(native.status,0,'native configured color tests failed');
   const line=native.stdout.split('\n').find(l=>l.startsWith('CONFIGURED_COLORS_REFERENCE '));assert(line);
   const row=JSON.parse(line.slice('CONFIGURED_COLORS_REFERENCE '.length));
   const formsLine=native.stdout.split('\n').find(l=>l.startsWith('CONFIGURED_COLORS_FORMS '));assert(formsLine);
   const forms=JSON.parse(formsLine.slice('CONFIGURED_COLORS_FORMS '.length));assert.equal(forms.length,result.forms.length);
   forms.forEach((row,i)=>{const ref=result.forms[i];assert.equal(row.token,ref.token);assert.equal(row.calls,ref.calls,row.token);assert(Math.abs(row.gray-ref.gray)<1e-3,row.token);});delete result.forms;
   assert(Math.abs(row.gray-result.gray)<1e-4);delete row.gray;delete result.gray;
   assert.deepEqual(row,result);
  }
  console.log('CONFIGURED_COLORS_REFERENCE '+JSON.stringify(result));
 }finally{await browser.close();}
}
main().catch(e=>{console.error(e);process.exitCode=1;});

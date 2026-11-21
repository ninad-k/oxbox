/** Reuse the project's ISC-licensed Lucide vectors without a runtime SVG dependency. */
import fs from 'node:fs';
import ts from 'typescript';
import {flatten} from './svg-lines.mjs';
const catalog=JSON.parse(fs.readFileSync('catalog/tools.json','utf8'));
const byId=new Map();
for(const filename of ['src/lib/registry.ts','src/lib/roadmap-registry.ts']){
  const file=ts.createSourceFile(filename,fs.readFileSync(filename,'utf8'),ts.ScriptTarget.Latest,true);
  const imports=new Map();
  for(const item of file.statements)if(ts.isImportDeclaration(item)&&ts.isStringLiteral(item.moduleSpecifier)&&item.moduleSpecifier.text.startsWith('@lucide/svelte/icons/'))imports.set(item.importClause?.name?.text,item.moduleSpecifier.text.split('/').at(-1));
  function visit(node){if(ts.isObjectLiteralExpression(node)){
    const props=new Map(node.properties.filter(ts.isPropertyAssignment).map(prop=>[prop.name.getText(file),prop.initializer]));
    const id=props.get('id'),icon=props.get('icon');
    if(id&&ts.isStringLiteral(id)&&icon&&ts.isIdentifier(icon)&&imports.has(icon.text))byId.set(id.text,imports.get(icon.text));
  }ts.forEachChild(node,visit);}visit(file);
}
const icons={};
for(const item of [...catalog.categories,...catalog.tools]){
  item.icon=byId.get(item.id)||item.icon;
  if(!item.icon)throw new Error('Missing icon binding for '+item.id);
  if(icons[item.icon])continue;
  const base='node_modules/@lucide/svelte/dist/icons/';
  let filename=base+item.icon+'.svelte';
  if(!fs.existsSync(filename)){
    const alias=fs.readFileSync(base+item.icon+'.js','utf8').match(/from "\.\/([a-z0-9-]+\.svelte)"/);
    if(!alias)throw new Error('Unsupported icon alias '+item.icon);
    filename=base+alias[1];
  }
  const source=fs.readFileSync(filename,'utf8');
  const start=source.indexOf('export const __iconData = ');
  const literal=source.slice(start+'export const __iconData = '.length).split(';')[0];
  if(start<0)throw new Error('Missing static icon data');
  const data=JSON.parse(literal);icons[item.icon]=flatten(data.node);
}
fs.writeFileSync('catalog/tools.json',JSON.stringify(catalog,null,2)+'\n');
fs.writeFileSync('catalog/icons.json',JSON.stringify({source:'@lucide/svelte 1.77.0; https://lucide.dev',license:'ISC',icons},null,2)+'\n');
console.log('Generated '+Object.keys(icons).length+' native icon vectors from the existing licensed package.');

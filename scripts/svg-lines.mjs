/** Flatten the small stroke-only SVG subset used by the project's existing Lucide icons. */
const point=(x,y)=>[Number(x),Number(y)];
function arc(from,values){
  let[rx,ry,rotation,large,sweep,x,y]=values;rx=Math.abs(rx);ry=Math.abs(ry);
  if(!rx||!ry||(x===from[0]&&y===from[1]))return[[x,y]];
  const angle=rotation*Math.PI/180,c=Math.cos(angle),s=Math.sin(angle),dx=(from[0]-x)/2,dy=(from[1]-y)/2;
  const xp=c*dx+s*dy,yp=-s*dx+c*dy,scale=xp*xp/(rx*rx)+yp*yp/(ry*ry);if(scale>1){rx*=Math.sqrt(scale);ry*=Math.sqrt(scale);}
  const numerator=rx*rx*ry*ry-rx*rx*yp*yp-ry*ry*xp*xp,denominator=rx*rx*yp*yp+ry*ry*xp*xp;
  const factor=(large===sweep?-1:1)*Math.sqrt(Math.max(0,numerator/denominator));
  const cxp=factor*rx*yp/ry,cyp=-factor*ry*xp/rx,cx=c*cxp-s*cyp+(from[0]+x)/2,cy=s*cxp+c*cyp+(from[1]+y)/2;
  const u=[(xp-cxp)/rx,(yp-cyp)/ry],v=[(-xp-cxp)/rx,(-yp-cyp)/ry];let begin=Math.atan2(u[1],u[0]),delta=Math.atan2(u[0]*v[1]-u[1]*v[0],u[0]*v[0]+u[1]*v[1]);
  if(sweep&&delta<0)delta+=2*Math.PI;if(!sweep&&delta>0)delta-=2*Math.PI;
  const steps=Math.max(2,Math.ceil(Math.abs(delta)*8));return Array.from({length:steps},(_,i)=>{const t=begin+delta*(i+1)/steps;return[cx+c*rx*Math.cos(t)-s*ry*Math.sin(t),cy+s*rx*Math.cos(t)+c*ry*Math.sin(t)];});
}
function pathLines(data){
  const tokens=data.match(/[A-Za-z]|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?/g)||[];
  const result=[];let line=[],p=[0,0],start=p,command='',index=0;
  const flush=()=>{if(line.length>1)result.push(line);line=[];};
  while(index<tokens.length){
    if(/^[A-Za-z]$/.test(tokens[index]))command=tokens[index++];
    const op=command.toUpperCase(),relative=command!==op;
    if(op==='Z'){line.push(start);p=start;flush();command='';continue;}
    const count={M:2,L:2,H:1,V:1,C:6,A:7}[op];if(!count||index+count>tokens.length)throw new Error('Unsupported SVG command '+command);
    const values=tokens.slice(index,index+count).map(Number);if(!values.every(Number.isFinite))throw new Error('Invalid SVG coordinate');index+=count;
    const dest=(x,y)=>relative?[p[0]+x,p[1]+y]:[x,y];
    if(op==='M'){flush();p=dest(...values);start=p;line=[p];command=relative?'l':'L';}
    else if(op==='L'){p=dest(...values);line.push(p);}
    else if(op==='H'){p=[relative?p[0]+values[0]:values[0],p[1]];line.push(p);}
    else if(op==='V'){p=[p[0],relative?p[1]+values[0]:values[0]];line.push(p);}
    else if(op==='A'){const end=dest(values[5],values[6]);line.push(...arc(p,[...values.slice(0,5),...end]));p=end;}
    else if(op==='C'){const a=p,b=dest(values[0],values[1]),c=dest(values[2],values[3]),d=dest(values[4],values[5]);for(let n=1;n<=16;n++){const t=n/16,q=1-t;line.push([q*q*q*a[0]+3*q*q*t*b[0]+3*q*t*t*c[0]+t*t*t*d[0],q*q*q*a[1]+3*q*q*t*b[1]+3*q*t*t*c[1]+t*t*t*d[1]]);}p=d;}
  }flush();return result;
}
export function flatten(nodes){
  return nodes.flatMap(([kind,attr])=>{
    if(kind==='path')return pathLines(attr.d);
    if(kind==='line')return[[point(attr.x1,attr.y1),point(attr.x2,attr.y2)]];
    if(kind==='circle'){const cx=Number(attr.cx),cy=Number(attr.cy),r=Number(attr.r);return[Array.from({length:49},(_,i)=>[cx+r*Math.cos(i*Math.PI/24),cy+r*Math.sin(i*Math.PI/24)])];}
    if(kind==='rect'){const x=Number(attr.x),y=Number(attr.y),w=Number(attr.width),h=Number(attr.height),r=Number(attr.rx||0);return pathLines(`M${x+r} ${y}H${x+w-r}a${r} ${r} 0 0 1 ${r} ${r}V${y+h-r}a${r} ${r} 0 0 1 ${-r} ${r}H${x+r}a${r} ${r} 0 0 1 ${-r} ${-r}V${y+r}a${r} ${r} 0 0 1 ${r} ${-r}Z`);}
    throw new Error('Unsupported icon shape '+kind);
  }).map(line=>line.map(point=>point.map(value=>Math.round(value*10000)/10000)));
}

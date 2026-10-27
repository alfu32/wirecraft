#!/usr/bin/env python3
"""Regenerate pure routing goldens from the recorded JVM source, without its UI.
Requires kotlinc/Java; WIRECRAFT_TEST_KOTLINC selects a compiler. Uses only temp files.
"""
import hashlib,json,os,pathlib,subprocess,tempfile
ROOT=pathlib.Path(__file__).resolve().parent.parent
REVISION='7d392d3a0e30b86445e00e7c03032a5188b26593'
SOURCE='app-desktop/src/main/kotlin/com/threadwork/app/ui/ThreadworkDesktopApp.kt'
source=subprocess.check_output(['git','-C',str(ROOT/'../threadworks'),'show',REVISION+':'+SOURCE],text=True)
methods=source[source.index('    private fun bestRouteBetween('):source.index('    private fun routeObstacles(')]
methods+=source[source.index('    private fun segmentsCross('):source.index('    private fun scheduleReroute(')]
methods+=source[source.index('    private fun portCapPoints('):source.index('    private fun drawDependencyAnnotations(')]
cases=[{'name':name,'start':[0,0],'end':end,'obstacles':obstacles,'occupied':occupied} for name,end,obstacles,occupied in [
 ('wide-diagonal',[400,240],[],[]),('tall-diagonal',[240,400],[],[]),
 ('descending-diagonal',[400,-240],[],[]),('reverse-diagonal',[-400,240],[],[]),
 ('pure-diagonal',[240,240],[],[]),('horizontal',[400,0],[],[]),
 ('vertical',[0,400],[],[]),('chamfered-obstacle-lane',[200,0],[[80,-30,40,60]],[]),('obstacle-switches-diagonal',[400,240],[[150,80,80,100]],[]),
 ('crossing-switches-diagonal',[400,240],[],[[[120,40],[120,200]]]),
]]
point=lambda p:f'Point({p[0]},{p[1]})'
caps=[{'side':side,'outgoing':outgoing} for outgoing in [True,False] for side in [1,-1]]
calls=[]
for c in cases:
 obstacles=','.join('Rectangle('+','.join(map(str,r))+')' for r in c['obstacles'])
 occupied=','.join(f'RouteSegment({point(a)},{point(b)},"other")' for a,b in c['occupied'])
 calls.append(f'println(router.solve({point(c["start"])},{point(c["end"])},listOf({obstacles}),listOf({occupied})).joinToString("; ") {{ "${{it.x}},${{it.y}}" }})')
for c in caps:
 calls.append('router.cap('+str(c['side'])+','+str(c['outgoing']).lower()+').let { (points,fill) -> println(fill.toString()+"|"+points.joinToString("; ") { "${it.first},${it.second}" }) }')
harness='''import java.awt.Point
import java.awt.Rectangle
import java.awt.geom.Line2D
import kotlin.math.*
typealias NodeId=String
data class RouteSegment(val a:Point,val b:Point,val linkId:NodeId)
class ReferenceRouter {
 companion object { const val ROUTING_STEP=40; const val ROUTING_CHAMFER=22; const val ROUTING_LANE_SPAN=7 }
 private fun compact(points:List<Point>)=points.fold(mutableListOf<Point>()) { acc,p -> if(acc.lastOrNull()!=p)acc+=p;acc }
 private fun Int.sign()=compareTo(0)
 private fun Rectangle.containsInclusive(p:Point)=p.x>=x&&p.x<=x+width&&p.y>=y&&p.y<=y+height
 fun solve(a:Point,b:Point,obstacles:List<Rectangle>,occupied:List<RouteSegment>)=bestRouteBetween(a,b,obstacles,occupied,"current",null)
 fun cap(side:Int,outgoing:Boolean)=portCapPoints(Point(100,100),side,outgoing)
'''+methods+'''\n}
fun main(){val router=ReferenceRouter()
'''+ '\n'.join(calls)+'\n}\n'
with tempfile.TemporaryDirectory(prefix='wirecraft-jvm-router-reference-') as temporary:
 temp=pathlib.Path(temporary);(temp/'ReferenceRouter.kt').write_text(harness)
 subprocess.run([os.environ.get('WIRECRAFT_TEST_KOTLINC','kotlinc'),str(temp/'ReferenceRouter.kt'),'-include-runtime','-d',str(temp/'reference.jar')],check=True,timeout=60)
 lines=subprocess.check_output(['java','-Djava.awt.headless=true','-jar',str(temp/'reference.jar')],text=True,timeout=15).splitlines()
 assert len(lines)==len(cases)+len(caps)
 for case,line in zip(cases,lines[:len(cases)]):case['expectedRoute']=[list(map(int,p.split(','))) for p in line.split('; ')]
 for cap,line in zip(caps,lines[len(cases):]):
  fill,points=line.split('|');cap['filled']=fill=='true';cap['points']=[list(map(float,p.split(','))) for p in points.split('; ')]
fixture={'sourceRevision':REVISION,'sourcePath':SOURCE,'extractedMethodsSha256':hashlib.sha256(methods.encode()).hexdigest(),'cases':cases,'portCaps':caps}
path=ROOT/'crates/wirecraft-core/tests/fixtures/router-reference.json';path.write_text(json.dumps(fixture,indent=2)+'\n');print('Generated',len(cases),'JVM routing goldens:',path)

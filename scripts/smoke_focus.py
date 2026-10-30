"""Keyboard focus policy shared by the native smokes.

Smokes run on the user's own display. They never activate or focus their window: when another
window holds the focus, the smoke pauses before sending input (xdotool sends to whatever window is
focused) and stops after a while instead of taking the focus back.
"""
import subprocess,sys,time
def focused(env):
 try:return int(subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True,stderr=subprocess.DEVNULL))
 except (subprocess.CalledProcessError,ValueError):return None
def await_focus(window,env,timeout=180):
 if focused(env)==int(window):return
 print(f'smoke paused: another window has the keyboard focus; click the smoke window {window} to resume',file=sys.stderr,flush=True)
 until=time.monotonic()+timeout
 while time.monotonic()<until:
  if focused(env)==int(window):
   time.sleep(.5);return
  time.sleep(.25)
 raise RuntimeError('the keyboard focus stayed on another window; the smoke stopped instead of taking it back')

import sys
import os
import json
import threading
import socket
import time

try:
    import dbus
    import dbus.service
    from dbus.mainloop.glib import DBusGMainLoop
    from gi.repository import GLib
    DBUS_AVAILABLE = True
except Exception as e:
    DBUS_AVAILABLE = False
    print(f"[mpris_bridge] DBus initialization failed: {e}", file=sys.stderr)

ipc_sock_path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/aethon_mpris_ipc.sock"
mpv_sock_path = sys.argv[2] if len(sys.argv) > 2 else ""

if DBUS_AVAILABLE:
    DBusGMainLoop(set_as_default=True)

class AethonMPRIS(dbus.service.Object if DBUS_AVAILABLE else object):
    def __init__(self, bus_name=None):
        if DBUS_AVAILABLE and bus_name:
            super().__init__(bus_name, '/org/mpris/MediaPlayer2')
        self.playback_status = 'Stopped'
        self.title = 'Aethon'
        self.artist = 'Aethon'
        self.album = 'Aethon'
        self.art_url = ''
        self.duration_us = 0
        self.position_us = 0
        self.volume = 1.0
        self.ipc_conn = None

    def set_ipc_conn(self, conn):
        self.ipc_conn = conn

    def send_to_mpv(self, cmd_dict):
        if not mpv_sock_path or not os.path.exists(mpv_sock_path):
            return
        try:
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.connect(mpv_sock_path)
            s.sendall((json.dumps(cmd_dict) + "\n").encode('utf-8'))
            s.close()
        except Exception:
            pass

    def send_to_ipc(self, msg_dict):
        if self.ipc_conn:
            try:
                self.ipc_conn.sendall((json.dumps(msg_dict) + "\n").encode('utf-8'))
            except Exception:
                pass

    if DBUS_AVAILABLE:
        @dbus.service.method('org.mpris.MediaPlayer2', out_signature='')
        def Raise(self):
            self.send_to_ipc({"event": "raise"})

        @dbus.service.method('org.mpris.MediaPlayer2', out_signature='')
        def Quit(self):
            self.send_to_ipc({"event": "quit"})

        @dbus.service.method('org.mpris.MediaPlayer2.Player', out_signature='')
        def PlayPause(self):
            self.send_to_mpv({"command": ["cycle", "pause"]})
            self.send_to_ipc({"event": "playpause"})

        @dbus.service.method('org.mpris.MediaPlayer2.Player', out_signature='')
        def Play(self):
            self.send_to_mpv({"command": ["set_property", "pause", False]})
            self.send_to_ipc({"event": "play"})

        @dbus.service.method('org.mpris.MediaPlayer2.Player', out_signature='')
        def Pause(self):
            self.send_to_mpv({"command": ["set_property", "pause", True]})
            self.send_to_ipc({"event": "pause"})

        @dbus.service.method('org.mpris.MediaPlayer2.Player', out_signature='')
        def Next(self):
            self.send_to_ipc({"event": "next"})

        @dbus.service.method('org.mpris.MediaPlayer2.Player', out_signature='')
        def Previous(self):
            self.send_to_ipc({"event": "prev"})

        @dbus.service.method('org.mpris.MediaPlayer2.Player', out_signature='')
        def Stop(self):
            self.send_to_mpv({"command": ["stop"]})
            self.send_to_ipc({"event": "stop"})

        @dbus.service.method('org.mpris.MediaPlayer2.Player', in_signature='x', out_signature='')
        def Seek(self, offset):
            secs = offset / 1000000.0
            self.send_to_mpv({"command": ["seek", secs, "relative"]})

        @dbus.service.method('org.mpris.MediaPlayer2.Player', in_signature='ox', out_signature='')
        def SetPosition(self, track_id, position):
            secs = position / 1000000.0
            self.send_to_mpv({"command": ["seek", secs, "absolute"]})

        @dbus.service.method('org.mpris.MediaPlayer2.Player', in_signature='s', out_signature='')
        def OpenUri(self, uri):
            pass

        @dbus.service.method('org.freedesktop.DBus.Properties', in_signature='s', out_signature='a{sv}')
        def GetAll(self, interface):
            if interface == 'org.mpris.MediaPlayer2':
                return {
                    'CanQuit': dbus.Boolean(True),
                    'CanRaise': dbus.Boolean(True),
                    'HasTrackList': dbus.Boolean(False),
                    'Identity': dbus.String('Aethon'),
                    'SupportedUriSchemes': dbus.Array([dbus.String('file'), dbus.String('http'), dbus.String('https')], signature='s'),
                    'SupportedMimeTypes': dbus.Array([dbus.String('audio/mpeg'), dbus.String('audio/flac'), dbus.String('audio/ogg'), dbus.String('audio/opus'), dbus.String('audio/mp4')], signature='s'),
                }
            elif interface == 'org.mpris.MediaPlayer2.Player':
                metadata = {
                    'mpris:trackid': dbus.ObjectPath('/org/mpris/MediaPlayer2/Track/1'),
                    'xesam:title': dbus.String(self.title),
                    'xesam:artist': dbus.Array([dbus.String(self.artist)], signature='s'),
                    'xesam:album': dbus.String(self.album),
                    'mpris:length': dbus.Int64(self.duration_us),
                }
                if self.art_url:
                    metadata['mpris:artUrl'] = dbus.String(self.art_url)

                return {
                    'PlaybackStatus': dbus.String(self.playback_status),
                    'Rate': dbus.Double(1.0),
                    'Metadata': dbus.Dictionary(metadata, signature='sv'),
                    'Volume': dbus.Double(self.volume),
                    'Position': dbus.Int64(self.position_us),
                    'CanControl': dbus.Boolean(True),
                    'CanPlay': dbus.Boolean(True),
                    'CanPause': dbus.Boolean(True),
                    'CanSeek': dbus.Boolean(True),
                    'CanGoNext': dbus.Boolean(True),
                    'CanGoPrevious': dbus.Boolean(True),
                }
            return {}

        @dbus.service.method('org.freedesktop.DBus.Properties', in_signature='ss', out_signature='v')
        def Get(self, interface, prop):
            all_props = self.GetAll(interface)
            return all_props.get(prop, '')

        @dbus.service.signal('org.freedesktop.DBus.Properties', signature='sa{sv}as')
        def PropertiesChanged(self, interface_name, changed_properties, invalidated_properties):
            pass

    def update_metadata(self, title, artist, album, art_url, duration_us, is_playing):
        self.title = title or 'Aethon'
        self.artist = artist or 'Unknown Artist'
        self.album = album or 'Aethon'
        self.art_url = art_url or ''
        self.duration_us = int(duration_us)
        self.playback_status = 'Playing' if is_playing else 'Paused'

        if DBUS_AVAILABLE:
            metadata = {
                'mpris:trackid': dbus.ObjectPath('/org/mpris/MediaPlayer2/Track/1'),
                'xesam:title': dbus.String(self.title),
                'xesam:artist': dbus.Array([dbus.String(self.artist)], signature='s'),
                'xesam:album': dbus.String(self.album),
                'mpris:length': dbus.Int64(self.duration_us),
            }
            if self.art_url:
                metadata['mpris:artUrl'] = dbus.String(self.art_url)

            changed = {
                'PlaybackStatus': dbus.String(self.playback_status),
                'Metadata': dbus.Dictionary(metadata, signature='sv'),
            }
            try:
                self.PropertiesChanged('org.mpris.MediaPlayer2.Player', changed, [])
            except Exception as e:
                print(f"[mpris_bridge] PropertiesChanged error: {e}", file=sys.stderr)

    def update_status(self, is_playing, position_us):
        new_status = 'Playing' if is_playing else 'Paused'
        self.position_us = int(position_us)
        if self.playback_status != new_status:
            self.playback_status = new_status
            if DBUS_AVAILABLE:
                try:
                    self.PropertiesChanged('org.mpris.MediaPlayer2.Player', {'PlaybackStatus': dbus.String(new_status)}, [])
                except Exception:
                    pass

def ipc_worker(mpris_obj):
    while True:
        try:
            if not os.path.exists(ipc_sock_path):
                time.sleep(0.2)
                continue
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect(ipc_sock_path)
            mpris_obj.set_ipc_conn(sock)
            f = sock.makefile('r')
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    data = json.loads(line)
                    cmd = data.get('action')
                    if cmd == 'update':
                        mpris_obj.update_metadata(
                            data.get('title', ''),
                            data.get('artist', ''),
                            data.get('album', ''),
                            data.get('art_url', ''),
                            data.get('duration_us', 0),
                            data.get('is_playing', False)
                        )
                    elif cmd == 'status':
                        mpris_obj.update_status(
                            data.get('is_playing', False),
                            data.get('position_us', 0)
                        )
                except Exception as ex:
                    print(f"[mpris_bridge] Error parsing IPC payload: {ex}", file=sys.stderr)
            sock.close()
        except Exception:
            time.sleep(0.5)

def main():
    mpris = None
    if DBUS_AVAILABLE:
        try:
            bus = dbus.SessionBus()
            bus_name = dbus.service.BusName('org.mpris.MediaPlayer2.aethon', bus)
            mpris = AethonMPRIS(bus_name)
            print("[mpris_bridge] Acquired org.mpris.MediaPlayer2.aethon", flush=True)
        except Exception as e:
            print(f"[mpris_bridge] Failed to acquire bus name: {e}", file=sys.stderr)
            mpris = AethonMPRIS()
    else:
        mpris = AethonMPRIS()

    t = threading.Thread(target=ipc_worker, args=(mpris,), daemon=True)
    t.start()

    if DBUS_AVAILABLE:
        loop = GLib.MainLoop()
        try:
            loop.run()
        except KeyboardInterrupt:
            pass
    else:
        while True:
            time.sleep(1)

if __name__ == '__main__':
    main()

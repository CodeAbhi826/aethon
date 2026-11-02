import sys
import os
import json
import threading
import socket
import time
import hashlib
import base64

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
parent_pid = int(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2].isdigit() else os.getppid()

if DBUS_AVAILABLE:
    DBusGMainLoop(set_as_default=True)


def normalize_art_url(art_url: str) -> str:
    if not art_url:
        return ""
    if art_url.startswith("data:image/"):
        try:
            header, b64_data = art_url.split(",", 1)
            ext = "png" if "png" in header else "jpg"
            digest = hashlib.sha256(b64_data[:4096].encode("utf-8")).hexdigest()[:16]
            runtime_dir = os.environ.get("XDG_RUNTIME_DIR") or "/tmp"
            out_path = os.path.join(runtime_dir, f"aethon_art_{digest}.{ext}")
            if not os.path.exists(out_path):
                with open(out_path, "wb") as f:
                    f.write(base64.b64decode(b64_data))
            return f"file://{out_path}"
        except Exception:
            return ""
    if art_url.startswith("/"):
        return f"file://{art_url}"
    return art_url


class AethonMPRIS(dbus.service.Object if DBUS_AVAILABLE else object):
    def __init__(self, bus_name=None):
        if DBUS_AVAILABLE and bus_name:
            super().__init__(bus_name, "/org/mpris/MediaPlayer2")
        self.playback_status = "Stopped"
        self.title = "Aethon"
        self.artist = "Aethon"
        self.album = "Aethon"
        self.art_url = ""
        self.duration_us = 0
        self.position_us = 0
        self.volume = 1.0
        self.ipc_conn = None
        self.lock = threading.Lock()

    def set_ipc_conn(self, conn):
        with self.lock:
            self.ipc_conn = conn

    def send_to_ipc(self, msg_dict):
        with self.lock:
            conn = self.ipc_conn
        if conn:
            try:
                conn.sendall((json.dumps(msg_dict) + "\n").encode("utf-8"))
            except Exception:
                pass

    if DBUS_AVAILABLE:
        @dbus.service.method("org.mpris.MediaPlayer2", out_signature="")
        def Raise(self):
            self.send_to_ipc({"event": "raise"})

        @dbus.service.method("org.mpris.MediaPlayer2", out_signature="")
        def Quit(self):
            self.send_to_ipc({"event": "quit"})

        @dbus.service.method("org.mpris.MediaPlayer2.Player", out_signature="")
        def PlayPause(self):
            self.send_to_ipc({"event": "playpause"})

        @dbus.service.method("org.mpris.MediaPlayer2.Player", out_signature="")
        def Play(self):
            self.send_to_ipc({"event": "play"})

        @dbus.service.method("org.mpris.MediaPlayer2.Player", out_signature="")
        def Pause(self):
            self.send_to_ipc({"event": "pause"})

        @dbus.service.method("org.mpris.MediaPlayer2.Player", out_signature="")
        def Next(self):
            self.send_to_ipc({"event": "next"})

        @dbus.service.method("org.mpris.MediaPlayer2.Player", out_signature="")
        def Previous(self):
            self.send_to_ipc({"event": "prev"})

        @dbus.service.method("org.mpris.MediaPlayer2.Player", out_signature="")
        def Stop(self):
            self.send_to_ipc({"event": "stop"})

        @dbus.service.method("org.mpris.MediaPlayer2.Player", in_signature="x", out_signature="")
        def Seek(self, offset):
            secs = float(offset) / 1_000_000.0
            self.send_to_ipc({"event": "seek", "offset": secs})

        @dbus.service.method("org.mpris.MediaPlayer2.Player", in_signature="ox", out_signature="")
        def SetPosition(self, track_id, position):
            secs = float(position) / 1_000_000.0
            self.send_to_ipc({"event": "set_position", "position": secs})

        @dbus.service.signal("org.mpris.MediaPlayer2.Player", signature="x")
        def Seeked(self, Position):
            pass

        @dbus.service.method("org.mpris.MediaPlayer2.Player", in_signature="s", out_signature="")
        def OpenUri(self, uri):
            self.send_to_ipc({"event": "open_uri", "uri": str(uri)})

        def _build_metadata(self):
            artists = [dbus.String(a.strip()) for a in self.artist.split(",") if a.strip()]
            if not artists:
                artists = [dbus.String(self.artist or "Unknown Artist")]
            metadata = {
                "mpris:trackid": dbus.ObjectPath("/org/mpris/MediaPlayer2/Track/1"),
                "xesam:title": dbus.String(self.title),
                "xesam:artist": dbus.Array(artists, signature="s"),
                "xesam:album": dbus.String(self.album),
                "mpris:length": dbus.Int64(self.duration_us),
            }
            clean_art = normalize_art_url(self.art_url)
            if clean_art:
                metadata["mpris:artUrl"] = dbus.String(clean_art)
            return dbus.Dictionary(metadata, signature="sv")

        @dbus.service.method("org.freedesktop.DBus.Properties", in_signature="s", out_signature="a{sv}")
        def GetAll(self, interface):
            if interface == "org.mpris.MediaPlayer2":
                return {
                    "CanQuit": dbus.Boolean(True),
                    "CanRaise": dbus.Boolean(True),
                    "HasTrackList": dbus.Boolean(False),
                    "Identity": dbus.String("Aethon"),
                    "DesktopEntry": dbus.String("aethon"),
                    "SupportedUriSchemes": dbus.Array(
                        [dbus.String("file"), dbus.String("http"), dbus.String("https")],
                        signature="s",
                    ),
                    "SupportedMimeTypes": dbus.Array(
                        [
                            dbus.String("audio/mpeg"),
                            dbus.String("audio/flac"),
                            dbus.String("audio/ogg"),
                            dbus.String("audio/opus"),
                            dbus.String("audio/mp4"),
                        ],
                        signature="s",
                    ),
                }
            elif interface == "org.mpris.MediaPlayer2.Player":
                return {
                    "PlaybackStatus": dbus.String(self.playback_status),
                    "Rate": dbus.Double(1.0),
                    "MinimumRate": dbus.Double(1.0),
                    "MaximumRate": dbus.Double(1.0),
                    "Metadata": self._build_metadata(),
                    "Volume": dbus.Double(self.volume),
                    "Position": dbus.Int64(self.position_us),
                    "LoopStatus": dbus.String("None"),
                    "Shuffle": dbus.Boolean(False),
                    "CanControl": dbus.Boolean(True),
                    "CanPlay": dbus.Boolean(True),
                    "CanPause": dbus.Boolean(True),
                    "CanSeek": dbus.Boolean(True),
                    "CanGoNext": dbus.Boolean(True),
                    "CanGoPrevious": dbus.Boolean(True),
                }
            return {}

        @dbus.service.method("org.freedesktop.DBus.Properties", in_signature="ss", out_signature="v")
        def Get(self, interface, prop):
            all_props = self.GetAll(interface)
            if prop in all_props:
                return all_props[prop]
            raise dbus.exceptions.DBusException(
                f"Property {prop} not found on {interface}",
                name="org.freedesktop.DBus.Error.InvalidArgs",
            )

        @dbus.service.method("org.freedesktop.DBus.Properties", in_signature="ssv", out_signature="")
        def Set(self, interface, prop, value):
            if interface == "org.mpris.MediaPlayer2.Player" and prop == "Volume":
                vol = max(0.0, min(1.0, float(value)))
                self.volume = vol
                self.send_to_ipc({"event": "volume", "volume": vol * 100.0})

        @dbus.service.signal("org.freedesktop.DBus.Properties", signature="sa{sv}as")
        def PropertiesChanged(self, interface_name, changed_properties, invalidated_properties):
            pass

    def update_metadata_main_thread(self, title, artist, album, art_url, duration_us, is_playing):
        self.title = title or "Aethon"
        self.artist = artist or "Unknown Artist"
        self.album = album or "Aethon"
        self.art_url = art_url or ""
        self.duration_us = int(duration_us)
        self.playback_status = "Playing" if is_playing else "Paused"

        if DBUS_AVAILABLE:
            changed = {
                "PlaybackStatus": dbus.String(self.playback_status),
                "Metadata": self._build_metadata(),
            }
            try:
                self.PropertiesChanged("org.mpris.MediaPlayer2.Player", changed, [])
            except Exception as e:
                print(f"[mpris_bridge] PropertiesChanged error: {e}", file=sys.stderr)
        return False

    def update_status_main_thread(self, is_playing, position_us, seeked=False):
        new_status = "Playing" if is_playing else "Paused"
        self.position_us = int(position_us)
        status_changed = self.playback_status != new_status
        self.playback_status = new_status
        if DBUS_AVAILABLE:
            if status_changed:
                try:
                    self.PropertiesChanged(
                        "org.mpris.MediaPlayer2.Player",
                        {"PlaybackStatus": dbus.String(new_status)},
                        [],
                    )
                except Exception:
                    pass
            if seeked:
                try:
                    self.Seeked(self.position_us)
                except Exception:
                    pass
        return False


def parent_alive() -> bool:
    try:
        os.kill(parent_pid, 0)
        return os.getppid() == parent_pid
    except OSError:
        return False


def ipc_worker(mpris_obj, loop):
    while parent_alive():
        try:
            if not os.path.exists(ipc_sock_path):
                time.sleep(0.2)
                continue
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect(ipc_sock_path)
            mpris_obj.set_ipc_conn(sock)
            with sock.makefile("r", encoding="utf-8", errors="replace") as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        data = json.loads(line)
                        cmd = data.get("action")
                        if cmd == "update":
                            args = (
                                data.get("title", ""),
                                data.get("artist", ""),
                                data.get("album", ""),
                                data.get("art_url", ""),
                                data.get("duration_us", 0),
                                data.get("is_playing", False),
                            )
                            if DBUS_AVAILABLE:
                                GLib.idle_add(mpris_obj.update_metadata_main_thread, *args)
                            else:
                                mpris_obj.update_metadata_main_thread(*args)
                        elif cmd == "status":
                            args = (
                                data.get("is_playing", False),
                                data.get("position_us", 0),
                                data.get("seeked", False),
                            )
                            if DBUS_AVAILABLE:
                                GLib.idle_add(mpris_obj.update_status_main_thread, *args)
                            else:
                                mpris_obj.update_status_main_thread(*args)
                        elif cmd == "shutdown":
                            sock.close()
                            if DBUS_AVAILABLE and loop:
                                GLib.idle_add(loop.quit)
                            os._exit(0)
                    except Exception as ex:
                        print(f"[mpris_bridge] Error parsing IPC payload: {ex}", file=sys.stderr)
            sock.close()
        except Exception:
            time.sleep(0.5)
    if DBUS_AVAILABLE and loop:
        GLib.idle_add(loop.quit)
    os._exit(0)


def main():
    loop = GLib.MainLoop() if DBUS_AVAILABLE else None
    if DBUS_AVAILABLE:
        try:
            bus = dbus.SessionBus()
            bus_name = dbus.service.BusName("org.mpris.MediaPlayer2.aethon", bus)
            mpris = AethonMPRIS(bus_name)
        except Exception as e:
            print(f"[mpris_bridge] Failed to acquire bus name: {e}", file=sys.stderr)
            mpris = AethonMPRIS()
    else:
        mpris = AethonMPRIS()

    t = threading.Thread(target=ipc_worker, args=(mpris, loop), daemon=True)
    t.start()

    if DBUS_AVAILABLE and loop:
        try:
            loop.run()
        except KeyboardInterrupt:
            pass
    else:
        while parent_alive():
            time.sleep(1)


if __name__ == "__main__":
    main()



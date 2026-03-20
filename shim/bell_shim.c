/*
 * bell_shim.c — LD_PRELOAD shim that intercepts gtk_widget_error_bell()
 * and routes it through soundthemed's D-Bus service.
 *
 * Build:
 *   gcc -shared -fPIC -O2 -o bell_shim.so bell_shim.c \
 *       $(pkg-config --cflags --libs dbus-1)
 */

#define _GNU_SOURCE
#include <dbus/dbus.h>
#include <dlfcn.h>
#include <stdio.h>

static void play_bell(void) {
    DBusError err;
    DBusConnection *conn;
    DBusMessage *msg;
    const char *event_id = "bell";

    dbus_error_init(&err);
    conn = dbus_bus_get(DBUS_BUS_SESSION, &err);
    if (!conn) {
        fprintf(stderr, "bell_shim: D-Bus connect failed: %s\n",
                err.message);
        dbus_error_free(&err);
        return;
    }

    msg = dbus_message_new_method_call(
        "org.freedesktop.SoundThemed1",
        "/org/freedesktop/SoundThemed1",
        "org.freedesktop.SoundThemed1",
        "PlaySound");

    if (!msg) {
        fprintf(stderr, "bell_shim: failed to create D-Bus message\n");
        dbus_connection_unref(conn);
        return;
    }

    dbus_message_set_no_reply(msg, TRUE);

    if (!dbus_message_append_args(msg,
            DBUS_TYPE_STRING, &event_id,
            DBUS_TYPE_INVALID)) {
        fprintf(stderr, "bell_shim: failed to append args\n");
        dbus_message_unref(msg);
        dbus_connection_unref(conn);
        return;
    }

    if (!dbus_connection_send(conn, msg, NULL)) {
        fprintf(stderr, "bell_shim: failed to send message\n");
    } else {
        dbus_connection_flush(conn);
    }

    dbus_message_unref(msg);
    dbus_connection_unref(conn);
}

void gtk_widget_error_bell(void *widget) {
    play_bell();
}

void gdk_display_beep(void *display) {
    play_bell();
}

void gdk_surface_beep(void *surface) {
    play_bell();
}

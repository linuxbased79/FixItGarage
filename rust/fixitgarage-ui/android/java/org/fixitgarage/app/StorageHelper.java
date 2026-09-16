package org.fixitgarage.app;

import android.content.Context;
import android.content.SharedPreferences;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;

/**
 * Durable state helpers for NativeActivity / Rust JNI.
 * SharedPreferences + atomic file write survive restarts better than
 * ad-hoc paths guessed from the dirs crate alone.
 */
public final class StorageHelper {
    private static final String PREFS = "fixitgarage_state";
    private static final String KEY_JSON = "state_json";
    private static final String KEY_VEHICLES = "vehicle_count";
    private static final String KEY_PATH = "last_path";

    private StorageHelper() {}

    public static String filesDir(Context ctx) {
        try {
            return ctx.getFilesDir().getAbsolutePath();
        } catch (Throwable t) {
            return "";
        }
    }

    public static String externalFilesDir(Context ctx) {
        try {
            File f = ctx.getExternalFilesDir(null);
            if (f == null) {
                return "";
            }
            return f.getAbsolutePath();
        } catch (Throwable t) {
            return "";
        }
    }

    /** Synchronous commit — apply() is too easy to lose on process kill. */
    public static boolean savePrefsBackup(Context ctx, String json, int vehicleCount, String path) {
        try {
            if (json == null) {
                json = "";
            }
            // SharedPreferences is not ideal for multi-MB blobs; garage JSON is small.
            if (json.length() > 900_000) {
                // Still store count so we know something was saved
                SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
                return p.edit()
                        .putInt(KEY_VEHICLES, vehicleCount)
                        .putString(KEY_PATH, path != null ? path : "")
                        .putString(KEY_JSON, "")
                        .commit();
            }
            SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
            return p.edit()
                    .putString(KEY_JSON, json)
                    .putInt(KEY_VEHICLES, vehicleCount)
                    .putString(KEY_PATH, path != null ? path : "")
                    .commit();
        } catch (Throwable t) {
            return false;
        }
    }

    public static String loadPrefsBackup(Context ctx) {
        try {
            SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
            String s = p.getString(KEY_JSON, "");
            return s != null ? s : "";
        } catch (Throwable t) {
            return "";
        }
    }

    public static int loadPrefsVehicleCount(Context ctx) {
        try {
            SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
            return p.getInt(KEY_VEHICLES, 0);
        } catch (Throwable t) {
            return 0;
        }
    }

    /** True if path is inside the app-private files directory (canonical). */
    public static boolean isUnderFilesDir(Context ctx, String path) {
        if (ctx == null || path == null || path.isEmpty()) {
            return false;
        }
        try {
            File files = ctx.getFilesDir().getCanonicalFile();
            File target = new File(path).getCanonicalFile();
            String root = files.getPath();
            String want = target.getPath();
            return want.equals(root) || want.startsWith(root + File.separator);
        } catch (Throwable t) {
            return false;
        }
    }

    /**
     * Atomic write: data → path.tmp → fsync → rename to path.
     * Refuses paths outside {@link Context#getFilesDir()}.
     */
    public static boolean writeFileAtomic(Context ctx, String path, byte[] data) {
        if (ctx == null || path == null || data == null) {
            return false;
        }
        if (!isUnderFilesDir(ctx, path)) {
            return false;
        }
        File out = new File(path);
        File tmp = new File(path + ".tmp");
        try {
            File parent = out.getParentFile();
            if (parent != null && !parent.exists() && !parent.mkdirs()) {
                return false;
            }
            FileOutputStream fos = new FileOutputStream(tmp);
            try {
                fos.write(data);
                fos.getFD().sync();
            } finally {
                fos.close();
            }
            if (out.exists() && !out.delete()) {
                // overwrite via rename may still work on some FS
            }
            if (!tmp.renameTo(out)) {
                // Fallback: copy then delete tmp
                FileOutputStream fos2 = new FileOutputStream(out);
                try {
                    FileInputStream fis = new FileInputStream(tmp);
                    try {
                        byte[] buf = new byte[8192];
                        int n;
                        while ((n = fis.read(buf)) > 0) {
                            fos2.write(buf, 0, n);
                        }
                        fos2.getFD().sync();
                    } finally {
                        fis.close();
                    }
                } finally {
                    fos2.close();
                }
                //noinspection ResultOfMethodCallIgnored
                tmp.delete();
            }
            return out.isFile() && out.length() == data.length;
        } catch (Throwable t) {
            try {
                //noinspection ResultOfMethodCallIgnored
                tmp.delete();
            } catch (Throwable ignored) {
            }
            return false;
        }
    }

    public static boolean writeFileAtomicUtf8(Context ctx, String path, String text) {
        if (text == null) {
            return false;
        }
        return writeFileAtomic(ctx, path, text.getBytes(StandardCharsets.UTF_8));
    }

    /**
     * After copying a camera capture into app-private storage: delete the
     * public MediaStore row and revoke grants so VIN/title photos do not stay
     * in the gallery.
     */
    public static void revokeAndDeleteUri(Context ctx, String uriStr) {
        if (ctx == null || uriStr == null || uriStr.isEmpty()) {
            return;
        }
        try {
            android.net.Uri uri = android.net.Uri.parse(uriStr);
            try {
                ctx.getContentResolver().delete(uri, null, null);
            } catch (Throwable ignored) {
            }
            int flags = android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION
                    | android.content.Intent.FLAG_GRANT_WRITE_URI_PERMISSION;
            try {
                ctx.revokeUriPermission(uri, flags);
            } catch (Throwable ignored) {
            }
        } catch (Throwable ignored) {
        }
    }
}

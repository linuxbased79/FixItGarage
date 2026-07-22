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

    /**
     * Atomic write: data → path.tmp → fsync → rename to path.
     * Returns true only if the final file exists and size matches.
     */
    public static boolean writeFileAtomic(String path, byte[] data) {
        if (path == null || data == null) {
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

    public static boolean writeFileAtomicUtf8(String path, String text) {
        if (text == null) {
            return false;
        }
        return writeFileAtomic(path, text.getBytes(StandardCharsets.UTF_8));
    }
}

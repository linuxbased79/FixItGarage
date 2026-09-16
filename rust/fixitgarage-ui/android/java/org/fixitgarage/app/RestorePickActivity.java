package org.fixitgarage.app;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.util.Log;

import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;

/**
 * SAF document picker for JSON backups. Copies the chosen file into
 * app-private {@code fig_restore.json} (2 MiB cap) then returns to the UI.
 */
public class RestorePickActivity extends Activity {
    private static final String TAG = "MotorNoterRestore";
    public static final String FILE_RESTORE = "fig_restore.json";
    private static final int REQ = 7;
    private static final long MAX_BYTES = 2L * 1024 * 1024;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Intent pick = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        pick.addCategory(Intent.CATEGORY_OPENABLE);
        pick.setType("*/*");
        pick.putExtra(Intent.EXTRA_MIME_TYPES, new String[] {
            "application/json", "text/plain", "application/octet-stream"
        });
        try {
            startActivityForResult(pick, REQ);
        } catch (Exception e) {
            Log.e(TAG, "open document failed", e);
            finish();
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode == REQ && resultCode == RESULT_OK && data != null) {
            Uri uri = data.getData();
            if (uri != null) {
                copyUri(uri);
            }
        }
        bringMainToFront();
        finish();
    }

    private void copyUri(Uri uri) {
        File out = new File(getFilesDir(), FILE_RESTORE);
        try (InputStream in = getContentResolver().openInputStream(uri);
             FileOutputStream fos = new FileOutputStream(out, false)) {
            if (in == null) {
                return;
            }
            byte[] buf = new byte[8192];
            int n;
            long total = 0;
            while ((n = in.read(buf)) > 0) {
                total += n;
                if (total > MAX_BYTES) {
                    Log.e(TAG, "restore file too large");
                    try {
                        fos.close();
                    } catch (Exception ignored) {
                    }
                    //noinspection ResultOfMethodCallIgnored
                    out.delete();
                    return;
                }
                fos.write(buf, 0, n);
            }
            fos.flush();
        } catch (Exception e) {
            Log.e(TAG, "copy restore failed", e);
        }
    }

    private void bringMainToFront() {
        Intent launch = getPackageManager().getLaunchIntentForPackage(getPackageName());
        if (launch == null) {
            return;
        }
        launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK
                | Intent.FLAG_ACTIVITY_CLEAR_TOP
                | Intent.FLAG_ACTIVITY_SINGLE_TOP
                | Intent.FLAG_ACTIVITY_REORDER_TO_FRONT);
        try {
            startActivity(launch);
        } catch (Exception e) {
            Log.e(TAG, "start main failed", e);
        }
    }
}

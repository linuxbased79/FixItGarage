package org.fixitgarage.app;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.util.Log;

import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;

/**
 * Receives shared text or images from OCR apps (Text Fairy, Lens, etc.) and
 * drops them into the app files dir for the native UI to consume:
 * <ul>
 *   <li>{@code fig_ocr_text.txt} — shared plain text</li>
 *   <li>{@code fig_ocr_image.jpg} — shared image copy</li>
 * </ul>
 * Then brings the main FixItGarage activity to the front.
 */
public class ShareReceiveActivity extends Activity {
    private static final String TAG = "FixItGarageShare";
    public static final String FILE_TEXT = "fig_ocr_text.txt";
    public static final String FILE_IMAGE = "fig_ocr_image.jpg";
    public static final String EXTRA_OCR = "org.fixitgarage.app.OCR_SHARE";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        try {
            handleIntent(getIntent());
        } catch (Exception e) {
            Log.e(TAG, "Failed to handle share intent", e);
        }
        bringMainToFront();
        finish();
    }

    private void handleIntent(Intent intent) throws Exception {
        if (intent == null) {
            return;
        }
        String action = intent.getAction();
        if (action == null) {
            return;
        }
        boolean isSend = Intent.ACTION_SEND.equals(action)
                || Intent.ACTION_SEND_MULTIPLE.equals(action)
                || Intent.ACTION_VIEW.equals(action);
        if (!isSend) {
            return;
        }

        String type = intent.getType();
        String text = intent.getStringExtra(Intent.EXTRA_TEXT);
        if (text != null && !text.trim().isEmpty()) {
            writeText(text.trim());
            Log.i(TAG, "Saved shared text (" + text.length() + " chars)");
        }

        Uri stream = null;
        if (Intent.ACTION_SEND_MULTIPLE.equals(action)) {
            // First image only
            java.util.ArrayList<Uri> list = intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM);
            if (list != null && !list.isEmpty()) {
                stream = list.get(0);
            }
        } else {
            stream = intent.getParcelableExtra(Intent.EXTRA_STREAM);
        }
        if (stream == null) {
            stream = intent.getData();
        }
        if (stream != null && (type == null || type.startsWith("image/") || type.startsWith("*/*")
                || type.equals("application/octet-stream"))) {
            // Prefer copying images; skip if we already have text-only share of a non-image
            if (type == null || type.startsWith("image/") || text == null || text.isEmpty()) {
                if (copyUriToImageFile(stream)) {
                    Log.i(TAG, "Saved shared image from " + stream);
                }
            }
        }
    }

    private void writeText(String text) throws Exception {
        File out = new File(getFilesDir(), FILE_TEXT);
        try (OutputStreamWriter w = new OutputStreamWriter(
                new FileOutputStream(out, false), StandardCharsets.UTF_8)) {
            w.write(text);
        }
    }

    private boolean copyUriToImageFile(Uri uri) {
        File out = new File(getFilesDir(), FILE_IMAGE);
        try (InputStream in = getContentResolver().openInputStream(uri);
             FileOutputStream fos = new FileOutputStream(out, false)) {
            if (in == null) {
                return false;
            }
            byte[] buf = new byte[8192];
            int n;
            long total = 0;
            while ((n = in.read(buf)) > 0) {
                fos.write(buf, 0, n);
                total += n;
                if (total > 25L * 1024 * 1024) {
                    // Cap ~25 MB
                    break;
                }
            }
            fos.flush();
            return total > 0;
        } catch (Exception e) {
            Log.e(TAG, "copy image failed: " + uri, e);
            return false;
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
        launch.putExtra(EXTRA_OCR, true);
        try {
            startActivity(launch);
        } catch (Exception e) {
            Log.e(TAG, "start main failed", e);
        }
    }
}

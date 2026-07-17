package org.fixitgarage.app;

import android.app.AlarmManager;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;
import android.util.Log;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;

/**
 * Re-registers FixItGarage date-based reminder alarms after reboot or app update.
 * Schedule is written by the native UI as {@code fig_alarms.json} in the app files dir.
 */
public class BootReceiver extends BroadcastReceiver {
    private static final String TAG = "FixItGarageBoot";
    private static final String ALARMS_FILE = "fig_alarms.json";
    private static final String EXTRA_DUE = "org.fixitgarage.app.DUE_CHECK";

    @Override
    public void onReceive(Context context, Intent intent) {
        if (intent == null) {
            return;
        }
        String action = intent.getAction();
        if (action == null) {
            return;
        }
        if (!Intent.ACTION_BOOT_COMPLETED.equals(action)
                && !Intent.ACTION_LOCKED_BOOT_COMPLETED.equals(action)
                && !Intent.ACTION_MY_PACKAGE_REPLACED.equals(action)
                && !"android.intent.action.QUICKBOOT_POWERON".equals(action)
                && !"com.htc.intent.action.QUICKBOOT_POWERON".equals(action)) {
            return;
        }
        Log.i(TAG, "Rescheduling alarms for action=" + action);
        try {
            rescheduleFromFile(context);
        } catch (Exception e) {
            Log.e(TAG, "Failed to reschedule alarms", e);
        }
    }

    private static void rescheduleFromFile(Context context) throws Exception {
        File file = new File(context.getFilesDir(), ALARMS_FILE);
        if (!file.isFile()) {
            Log.i(TAG, "No alarm schedule file yet: " + file.getAbsolutePath());
            return;
        }
        String json = readFile(file);
        if (json == null || json.trim().isEmpty()) {
            return;
        }
        JSONArray arr = new JSONArray(json);
        AlarmManager am = (AlarmManager) context.getSystemService(Context.ALARM_SERVICE);
        if (am == null) {
            Log.e(TAG, "AlarmManager unavailable");
            return;
        }
        PackageManager pm = context.getPackageManager();
        Intent launch = pm.getLaunchIntentForPackage(context.getPackageName());
        if (launch == null) {
            Log.e(TAG, "No launch intent");
            return;
        }
        launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK
                | Intent.FLAG_ACTIVITY_CLEAR_TOP
                | Intent.FLAG_ACTIVITY_SINGLE_TOP);
        launch.putExtra(EXTRA_DUE, true);

        long now = System.currentTimeMillis();
        int scheduled = 0;
        for (int i = 0; i < arr.length() && i < 24; i++) {
            JSONObject o = arr.getJSONObject(i);
            int code = o.optInt("code", 0);
            long dueMs = o.optLong("due_ms", 0L);
            if (code == 0 || dueMs <= now) {
                continue;
            }
            int flags = PendingIntent.FLAG_UPDATE_CURRENT;
            if (Build.VERSION.SDK_INT >= 23) {
                flags |= PendingIntent.FLAG_IMMUTABLE;
            }
            PendingIntent pi = PendingIntent.getActivity(context, code, launch, flags);
            try {
                if (Build.VERSION.SDK_INT >= 23) {
                    am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, dueMs, pi);
                } else {
                    am.set(AlarmManager.RTC_WAKEUP, dueMs, pi);
                }
                scheduled++;
            } catch (Exception e) {
                Log.w(TAG, "set alarm code=" + code + " failed: " + e.getMessage());
                try {
                    am.set(AlarmManager.RTC_WAKEUP, dueMs, pi);
                    scheduled++;
                } catch (Exception e2) {
                    Log.e(TAG, "fallback set failed", e2);
                }
            }
        }
        Log.i(TAG, "Rescheduled " + scheduled + " reminder alarm(s)");
    }

    private static String readFile(File file) throws Exception {
        StringBuilder sb = new StringBuilder();
        try (BufferedReader br = new BufferedReader(
                new InputStreamReader(new FileInputStream(file), StandardCharsets.UTF_8))) {
            String line;
            while ((line = br.readLine()) != null) {
                sb.append(line).append('\n');
            }
        }
        return sb.toString();
    }
}

package com.jondo.jondo;

import android.app.Activity;
import android.content.Intent;

/**
 * The activity Jondo launches with.
 *
 * <p>Extending {@link dev.gpui.mobile.GpuiActivity} keeps the splash hold, the
 * native library loading and the IME bridge; overriding {@code onActivityResult}
 * is what lets a SAF picker answer Rust, because {@code NativeActivity} drops
 * that callback.
 */
public class MainActivity extends dev.gpui.mobile.GpuiActivity {
    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        SyncDocuments.picked(this, requestCode, resultCode, data);
    }

    /**
     * Leaves the app from its first page the way the system back gesture does.
     *
     * <p>GPUI consumes every key event, so the back press never reaches
     * {@code onBackPressed}; Rust calls this instead of finishing the task, so
     * playback keeps running and the task stays in recents.
     */
    public static String background(Activity activity) {
        if (activity == null) return "no activity";
        activity.runOnUiThread(() -> activity.moveTaskToBack(true));
        return "";
    }

    public static String preferences(Activity activity, String pauseOnDisconnect, String pauseOnFocus) {
        dev.gpui.mobile.GpuiMediaSession.setPreferences(
                "1".equals(pauseOnDisconnect),
                "1".equals(pauseOnFocus)
        );
        return "";
    }
}

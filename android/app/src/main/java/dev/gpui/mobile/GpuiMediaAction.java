package dev.gpui.mobile;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/**
 * Turns the notification's transport buttons into the same actions the media
 * session callbacks raise.
 *
 * <p>{@code MediaButtonReceiver} cannot deliver these: it looks for a service
 * that handles {@code ACTION_MEDIA_BUTTON} or a media browser service, and a
 * NativeActivity app has neither. This receiver is the action target instead,
 * so it reports straight to the JNI bridge.
 */
public class GpuiMediaAction extends BroadcastReceiver {

    static final String EXTRA = "dev.gpui.mobile.MEDIA_ACTION";

    @Override
    public void onReceive(Context context, Intent intent) {
        String action = intent != null ? intent.getStringExtra(EXTRA) : null;
        if (action != null) {
            GpuiMediaSession.dispatch(action);
        }
    }
}

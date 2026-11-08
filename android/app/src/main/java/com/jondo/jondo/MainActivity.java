package com.jondo.jondo;

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
}

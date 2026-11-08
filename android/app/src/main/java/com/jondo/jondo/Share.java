package com.jondo.jondo;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;

import androidx.core.content.FileProvider;

import java.io.File;

/**
 * Hands a private file to another app, or to the system package installer.
 *
 * <p>Both leave {@code filesDir} through the {@code FileProvider} declared in
 * the manifest ({@code <applicationId>.files}), which is the only way a
 * {@code content://} URI can be granted to an app that cannot read the app's
 * private storage. Every method answers an empty string on success and the
 * platform message otherwise, like {@link SyncDocuments}.
 */
public final class Share {
    private Share() {}

    private static Uri uri(Activity activity, String path) {
        return FileProvider.getUriForFile(activity,
                activity.getPackageName() + ".files", new File(path));
    }

    /** Opens the system share chooser on the file. */
    public static String send(Activity activity, String path, String mime, String title) {
        if (activity == null || path == null || path.isEmpty()) return "no file";
        try {
            Intent intent = new Intent(Intent.ACTION_SEND)
                    .setType(mime == null || mime.isEmpty() ? "*/*" : mime)
                    .putExtra(Intent.EXTRA_STREAM, uri(activity, path))
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            activity.startActivity(Intent.createChooser(intent, title));
            return "";
        } catch (Exception e) {
            return e.getMessage() == null ? "share failed" : e.getMessage();
        }
    }

    /** Opens the system package installer on a downloaded APK. */
    public static String install(Activity activity, String path) {
        if (activity == null || path == null || path.isEmpty()) return "no file";
        try {
            Intent intent = new Intent(Intent.ACTION_VIEW)
                    .setDataAndType(uri(activity, path),
                            "application/vnd.android.package-archive")
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            activity.startActivity(intent);
            return "";
        } catch (Exception e) {
            return e.getMessage() == null ? "install failed" : e.getMessage();
        }
    }
}

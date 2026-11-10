package com.jondo.jondo;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.provider.DocumentsContract;
import android.provider.OpenableColumns;
import android.util.Log;

import org.json.JSONObject;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;

/**
 * SAF document access for the backup document: pick a location, stage the
 * provider's copy for Rust, and write Rust's answer back.
 *
 * <p>This is the Android half of {@code src/backup.rs}, and it follows Jota's
 * own {@code SyncDocuments} so both apps read and write the same document the
 * same way. The document never crosses JNI as a string: the provider's copy is
 * streamed into {@code filesDir/sync/incoming.json}, Rust merges and leaves
 * {@code outgoing.json}, and that is streamed back with {@code "wt"} so a
 * shorter document cannot keep the previous tail.
 *
 * <p>Every document call is blocking on purpose: Rust runs it on a worker
 * thread, never on GPUI's event loop. Only the picker launch is posted to the
 * activity's own UI thread, where the framework accepts an activity start. A
 * picker answer is delivered by {@link MainActivity#onActivityResult} into
 * {@link #picked}.
 */
public final class SyncDocuments {
    private static final String TAG = "SyncDocuments";

    /** Must match the request code MainActivity routes back here. */
    static final int PICK_REQUEST = 7013;

    /** Must match {@code backup::FILE} in the Rust backend. */
    private static final String BACKUP = "jota.backup.json";

    /** Same staged names as {@code INCOMING} and {@code OUTGOING} in backup.rs. */
    private static final String INCOMING = "incoming.json";
    private static final String OUTGOING = "outgoing.json";

    /** Which picker the pending answer belongs to. */
    private static final int FOLDER = 0;
    private static final int PICK = 1;
    private static final int CREATE = 2;

    /** Null while a picker is open, empty when it was cancelled, else the URI. */
    private static volatile String picked;
    private static volatile int kind;

    /** A launch that failed on the UI thread, answered by the next {@link #take}. */
    private static volatile String failed;

    private SyncDocuments() {}

    /**
     * Open the system folder picker. The document inside the chosen folder is
     * answered by {@link #take}, which creates it when the folder holds none.
     *
     * <p>The picker is a tree, not a document: it lists every DocumentsProvider
     * (Drive, Dropbox, OneDrive and local storage), and the persistable grant
     * taken over the tree is what keeps the document inside it readable later.
     */
    public static String folder(Activity activity) {
        return start(activity, FOLDER, new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
                .addFlags(Intent.FLAG_GRANT_PREFIX_URI_PERMISSION));
    }

    /**
     * Open the system document picker on an existing backup. A tree picker
     * cannot reach Drive, whose provider announces no tree support, so the cloud
     * is reached one document at a time.
     */
    public static String pick(Activity activity, String last) {
        return start(activity, PICK, document(activity, Intent.ACTION_OPEN_DOCUMENT, last));
    }

    /** Create the backup document through the system picker. */
    public static String create(Activity activity, String last) {
        return start(activity, CREATE, document(activity, Intent.ACTION_CREATE_DOCUMENT, last)
                .putExtra(Intent.EXTRA_TITLE, BACKUP));
    }

    /**
     * The pending pick's answer, consumed once. Empty while the picker is still
     * open, {@code {"uri":""}} when it was cancelled, an error object when the
     * provider refused, else the usual answer.
     *
     * <p>The launch never blocks: the answer is polled from Rust instead, like
     * the rest of the document calls.
     */
    public static String take(Activity activity) {
        String failure = failed;
        if (failure != null) {
            failed = null;
            picked = null;
            return failure;
        }

        String uri = picked;
        if (uri == null) return "";
        picked = null;
        try {
            if (uri.isEmpty()) return "{\"uri\":\"\"}";
            if (kind == FOLDER) return resolve(activity, Uri.parse(uri));
            return answer(activity, uri, kind == CREATE);
        } catch (RuntimeException e) {
            Log.e(TAG, "picker answer failed", e);
            return error(String.valueOf(e));
        }
    }

    /**
     * Starts the picker and answers {@code ""} when it is on its way.
     *
     * <p>The launch is posted to the activity's UI thread: Rust calls this from
     * GPUI's native event loop, and the framework refuses a picker started from
     * any other thread with {@link IllegalMonitorStateException}. A launch that
     * fails there is kept for {@link #take}, which runs on the caller's worker.
     */
    private static String start(Activity activity, int which, Intent intent) {
        if (activity == null) return error("no activity");
        picked = null;
        failed = null;
        kind = which;
        Intent request = intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION
                | Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
        activity.runOnUiThread(() -> {
            try {
                activity.startActivityForResult(request, PICK_REQUEST);
            } catch (RuntimeException e) {
                Log.e(TAG, "picker failed", e);
                failed = error(String.valueOf(e));
            }
        });
        return "";
    }

    private static Intent document(Activity activity, String action, String last) {
        Intent intent = new Intent(action)
                .addCategory(Intent.CATEGORY_OPENABLE)
                .setType("application/json");
        if (Build.VERSION.SDK_INT >= 26) {
            String initial = last == null || last.isEmpty() ? cloudRoot(activity) : last;
            if (initial != null) {
                intent.putExtra(DocumentsContract.EXTRA_INITIAL_URI, Uri.parse(initial));
            }
        }
        return intent;
    }

    /**
     * The cloud provider's first writable root, so the picker lands in Drive
     * instead of "Recent". Any failure answers null and the picker falls back to
     * its default location.
     *
     * shortcut: the root is picked by the provider's own flags, which is
     * heuristic; narrow it to a known root id when a provider misbehaves.
     */
    private static String cloudRoot(Activity activity) {
        if (activity == null) return null;
        String authority = "com.google.android.apps.docs.storage";
        try {
            android.database.Cursor cursor = activity.getContentResolver()
                    .query(DocumentsContract.buildRootsUri(authority),
                            new String[]{DocumentsContract.Root.COLUMN_DOCUMENT_ID,
                                    DocumentsContract.Root.COLUMN_FLAGS},
                            null, null, null);
            if (cursor == null) return null;
            try {
                while (cursor.moveToNext()) {
                    long flags = cursor.getLong(1);
                    if ((flags & DocumentsContract.Root.FLAG_SUPPORTS_CREATE) == 0) continue;
                    String id = cursor.getString(0);
                    if (id == null || id.isEmpty()) continue;
                    return DocumentsContract.buildDocumentUri(authority, id).toString();
                }
            } finally {
                cursor.close();
            }
        } catch (Exception e) {
            Log.w(TAG, "no cloud root", e);
        }
        return null;
    }

    private static String resolve(Activity activity, Uri tree) {
        if (activity == null) return error("no activity");
        try {
            String treeId = DocumentsContract.getTreeDocumentId(tree);
            Uri parent = DocumentsContract.buildDocumentUriUsingTree(tree, treeId);
            Uri existing = child(activity, tree, treeId);
            if (existing != null) return answer(activity, existing.toString(), false);
            Uri created = DocumentsContract.createDocument(
                    activity.getContentResolver(), parent, "application/json", BACKUP);
            if (created == null) return error("cannot create " + BACKUP);
            // createDocument may answer a plain document URI, while the grant is
            // held on the tree, so the URI is rebuilt onto it.
            String documentId = DocumentsContract.getDocumentId(created);
            return answer(activity, DocumentsContract.buildDocumentUriUsingTree(tree, documentId).toString(), true);
        } catch (Exception e) {
            Log.e(TAG, "resolve failed", e);
            return error(e.getMessage());
        }
    }

    /** The backup document among the folder's children, or null when absent. */
    private static Uri child(Activity activity, Uri tree, String parentId) {
        Uri children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, parentId);
        android.database.Cursor cursor = activity.getContentResolver().query(children,
                new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                        DocumentsContract.Document.COLUMN_DISPLAY_NAME},
                null, null, null);
        if (cursor == null) return null;
        try {
            while (cursor.moveToNext()) {
                String name = cursor.getString(1);
                if (name != null && name.trim().equalsIgnoreCase(BACKUP)) {
                    return DocumentsContract.buildDocumentUriUsingTree(tree, cursor.getString(0));
                }
            }
        } finally {
            cursor.close();
        }
        return null;
    }

    /**
     * Copy the provider's document into {@code incoming.json} and answer its
     * metadata as {@code {"lastModified":ms,"size":bytes,"displayName":"..."}}.
     * A null answer is an unreadable document, which Rust reports instead of
     * keeping a location that fails on every later pass.
     *
     * <p>The copy is streamed and the metadata comes from the same pass, so a
     * provider behind the SAF cache is asked once and not twice.
     */
    public static String stage(Activity activity, String uriStr) {
        if (activity == null || uriStr == null || uriStr.isEmpty()) return null;
        File dir = stageDir(activity);
        if (dir == null) return null;
        try {
            InputStream is = activity.getContentResolver().openInputStream(Uri.parse(uriStr));
            if (is == null) return null;
            try {
                OutputStream os = new FileOutputStream(new File(dir, INCOMING));
                try {
                    byte[] buffer = new byte[64 * 1024];
                    int read;
                    while ((read = is.read(buffer)) != -1) os.write(buffer, 0, read);
                    os.flush();
                } finally {
                    os.close();
                }
            } finally {
                is.close();
            }
        } catch (Exception e) {
            Log.e(TAG, "stage failed", e);
            return null;
        }
        return info(activity, uriStr);
    }

    /**
     * Write {@code outgoing.json} back to the provider and drop the staged copy
     * Rust read. Returns empty on success, the platform message otherwise.
     *
     * <p>The stream is opened in {@code "wt"} mode so the copy is truncated:
     * a shorter document written over a longer one would otherwise keep the old
     * tail and leave a document no other device can parse.
     *
     * <p>{@code outgoing.json} stays behind on purpose: Rust fingerprints the
     * bytes that were just committed and removes the file itself, so the write
     * baseline only moves once the provider really holds the document.
     */
    public static String commit(Activity activity, String uriStr) {
        if (activity == null || uriStr == null || uriStr.isEmpty()) return "empty uri";
        File dir = stageDir(activity);
        if (dir == null) return "no stage dir";
        File incoming = new File(dir, INCOMING);
        File outgoing = new File(dir, OUTGOING);
        try {
            OutputStream os = activity.getContentResolver()
                    .openOutputStream(Uri.parse(uriStr), "wt");
            if (os == null) return "cannot open output stream";
            try {
                InputStream is = new FileInputStream(outgoing);
                try {
                    byte[] buffer = new byte[64 * 1024];
                    int read;
                    while ((read = is.read(buffer)) != -1) os.write(buffer, 0, read);
                    os.flush();
                } finally {
                    is.close();
                }
            } finally {
                os.close();
            }
        } catch (Exception e) {
            Log.e(TAG, "commit failed", e);
            // The staged files stay and Rust's baseline does not move, so the next
            // pass merges again and writes the document out.
            return e.getMessage();
        }
        incoming.delete();
        return "";
    }

    /**
     * Document metadata as {@code {"lastModified":ms,"size":bytes}}. A SAF/cloud
     * provider answers from its own cache and offers no way to force a refresh,
     * so callers treat an old {@code lastModified} (or a 0) as "not fresh"
     * instead of writing on top of it. Returns {@code {}} when unknown.
     */
    public static String info(Activity activity, String uriStr) {
        if (activity == null || uriStr == null || uriStr.isEmpty()) return "{}";
        try {
            android.database.Cursor cursor = activity.getContentResolver()
                    .query(Uri.parse(uriStr), null, null, null, null);
            if (cursor == null) return "{}";
            try {
                if (!cursor.moveToFirst()) return "{}";
                long modified = 0;
                long size = -1;
                String displayName = "";
                int modIndex = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_LAST_MODIFIED);
                if (modIndex >= 0 && !cursor.isNull(modIndex)) modified = cursor.getLong(modIndex);
                int sizeIndex = cursor.getColumnIndex(OpenableColumns.SIZE);
                if (sizeIndex >= 0 && !cursor.isNull(sizeIndex)) size = cursor.getLong(sizeIndex);
                int nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (nameIndex >= 0 && !cursor.isNull(nameIndex)) displayName = cursor.getString(nameIndex);
                return new JSONObject()
                        .put("lastModified", modified)
                        .put("size", size)
                        .put("displayName", displayName)
                        .toString();
            } finally {
                cursor.close();
            }
        } catch (Exception e) {
            Log.e(TAG, "info failed", e);
            return "{}";
        }
    }

    /** The dir Rust reads and writes as well: the app files dir plus {@code /sync}. */
    private static File stageDir(Activity activity) {
        File dir = new File(activity.getFilesDir(), "sync");
        if (dir.isDirectory() || dir.mkdirs()) return dir;
        Log.e(TAG, "cannot create stage dir");
        return null;
    }

    /**
     * The picker's answer, with the provider's name for the document: a created
     * file may land under a duplicate name, and the name is the only way the
     * caller can tell.
     */
    private static String answer(Activity activity, String uri, boolean created) {
        String name = "";
        try {
            name = new JSONObject(info(activity, uri)).optString("displayName", "");
        } catch (Exception e) {
            Log.w(TAG, "no display name", e);
        }
        try {
            return new JSONObject().put("uri", uri).put("created", created)
                    .put("displayName", name).toString();
        } catch (Exception e) {
            return "{\"uri\":\"\",\"error\":\"cannot encode answer\"}";
        }
    }

    private static String error(String message) {
        try {
            return new JSONObject().put("uri", "")
                    .put("error", message == null ? "unknown" : message).toString();
        } catch (Exception e) {
            return "{\"uri\":\"\",\"error\":\"unknown\"}";
        }
    }

    /** Result of the picker, forwarded by MainActivity.onActivityResult. */
    static void picked(Activity activity, int requestCode, int resultCode, Intent data) {
        if (requestCode != PICK_REQUEST) return;
        try {
            if (activity != null && resultCode == Activity.RESULT_OK
                    && data != null && data.getData() != null) {
                Uri uri = data.getData();
                int granted = data.getFlags()
                        & (Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
                try {
                    activity.getContentResolver().takePersistableUriPermission(uri, granted);
                } catch (Exception ignored) {
                }
                picked = uri.toString();
            } else {
                picked = "";
            }
        } catch (Exception e) {
            Log.e(TAG, "picked failed", e);
            picked = "";
        }
    }
}

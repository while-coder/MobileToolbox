package com.mobiletoolbox.tools;

import java.lang.reflect.Method;
import java.util.List;

/** Runs as adb shell via app_process; no Android app installation is required. */
public final class AppLabels {
    private static void quote(StringBuilder output, String value) {
        output.append('"');
        for (int i = 0; i < value.length(); i++) {
            char ch = value.charAt(i);
            if (ch == '"' || ch == '\\') output.append('\\').append(ch);
            else if (ch < 32) output.append(String.format("\\u%04x", (int) ch));
            else output.append(ch);
        }
        output.append('"');
    }

    public static void main(String[] args) throws Exception {
        Class<?> looper = Class.forName("android.os.Looper");
        looper.getMethod("prepareMainLooper").invoke(null);
        Class<?> activityThread = Class.forName("android.app.ActivityThread");
        Object thread = activityThread.getMethod("systemMain").invoke(null);
        Object context = activityThread.getMethod("getSystemContext").invoke(thread);
        Object manager = Class.forName("android.content.Context").getMethod("getPackageManager").invoke(context);
        Class<?> pm = Class.forName("android.content.pm.PackageManager");
        Class<?> applicationInfo = Class.forName("android.content.pm.ApplicationInfo");
        List<?> applications = (List<?>) pm.getMethod("getInstalledApplications", int.class).invoke(manager, 0);
        Method getLabel = pm.getMethod("getApplicationLabel", applicationInfo);
        StringBuilder output = new StringBuilder("{");
        boolean first = true;
        for (Object app : applications) {
            String packageName = (String) applicationInfo.getField("packageName").get(app);
            try {
                CharSequence label = (CharSequence) getLabel.invoke(manager, app);
                if (label == null || label.toString().trim().isEmpty()) continue;
                if (!first) output.append(',');
                first = false;
                quote(output, packageName);
                output.append(':');
                quote(output, label.toString());
            } catch (Exception ignored) {
                // One unreadable application must not prevent other labels from loading.
            }
        }
        System.out.println(output.append('}'));
        System.exit(0);
    }
}

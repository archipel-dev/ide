//! Android app templates for the New Project wizard (Gradle, Kotlin DSL, Kotlin sources).
//! Plugin and library versions are fixed here; nothing is downloaded at generation time.
use std::path::PathBuf;

/// What the generated app module contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidActivity {
    /// `MainActivity` using Jetpack Compose ("Empty Activity").
    Compose,
    /// `MainActivity` using a View layout ("Empty Views Activity").
    Views,
    /// A manifest without any activity ("No Activity").
    None,
}

const AGP: &str = "8.7.3";
const KOTLIN: &str = "2.0.21";

/// The application id / package: `group` plus the project name reduced to `[a-z0-9]`.
pub fn application_id(group: &str, name: &str) -> String {
    let mut tail: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if tail.is_empty() || tail.starts_with(|c: char| c.is_ascii_digit()) {
        tail.insert_str(0, "app");
    }
    format!("{group}.{tail}")
}

pub(crate) fn files(
    files: &mut Vec<(PathBuf, String)>,
    name: &str,
    group: &str,
    activity: AndroidActivity,
) {
    let package = application_id(group, name);
    let fill = |text: &str| {
        text.replace("@PKG@", &package)
            .replace("@NAME@", name)
            .replace("@AGP@", AGP)
            .replace("@KOTLIN@", KOTLIN)
    };
    let compose = activity == AndroidActivity::Compose;
    let mut add = |path: &str, text: &str| files.push((path.into(), fill(text)));
    add(
        "settings.gradle.kts",
        r#"pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "@NAME@"
include(":app")
"#,
    );
    add(
        "build.gradle.kts",
        if compose {
            r#"plugins {
    id("com.android.application") version "@AGP@" apply false
    id("org.jetbrains.kotlin.android") version "@KOTLIN@" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "@KOTLIN@" apply false
}
"#
        } else {
            r#"plugins {
    id("com.android.application") version "@AGP@" apply false
    id("org.jetbrains.kotlin.android") version "@KOTLIN@" apply false
}
"#
        },
    );
    add(
        "gradle.properties",
        "org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8\nandroid.useAndroidX=true\nkotlin.code.style=official\nandroid.nonTransitiveRClass=true\n",
    );
    let (plugin, features, deps) = match activity {
        AndroidActivity::Compose => (
            "    id(\"org.jetbrains.kotlin.plugin.compose\")\n",
            "    buildFeatures {\n        compose = true\n    }\n",
            r#"    implementation(platform("androidx.compose:compose-bom:2024.10.01"))
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    debugImplementation("androidx.compose.ui:ui-tooling")
"#,
        ),
        AndroidActivity::Views => (
            "",
            "",
            r#"    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.constraintlayout:constraintlayout:2.2.0")
"#,
        ),
        AndroidActivity::None => (
            "",
            "",
            "    implementation(\"androidx.core:core-ktx:1.13.1\")\n",
        ),
    };
    add(
        "app/build.gradle.kts",
        &format!(
            r#"plugins {{
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
{plugin}}}

android {{
    namespace = "@PKG@"
    compileSdk = 35

    defaultConfig {{
        applicationId = "@PKG@"
        minSdk = 24
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"
    }}

    buildTypes {{
        release {{
            isMinifyEnabled = false
        }}
    }}
    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }}
    kotlinOptions {{
        jvmTarget = "17"
    }}
{features}}}

dependencies {{
{deps}}}
"#
        ),
    );
    add(
        "app/proguard-rules.pro",
        "# Add project specific ProGuard rules here.\n",
    );
    add(
        "app/src/main/res/values/strings.xml",
        "<resources>\n    <string name=\"app_name\">@NAME@</string>\n</resources>\n",
    );
    let theme = if compose {
        "android:Theme.Material.Light.NoActionBar"
    } else {
        "Theme.Material3.DayNight.NoActionBar"
    };
    add(
        "app/src/main/res/values/themes.xml",
        &format!(
            "<resources>\n    <style name=\"Theme.App\" parent=\"{theme}\" />\n</resources>\n"
        ),
    );
    let activity_xml = if activity == AndroidActivity::None {
        ""
    } else {
        "        <activity\n            android:name=\".MainActivity\"\n            android:exported=\"true\">\n            <intent-filter>\n                <action android:name=\"android.intent.action.MAIN\" />\n                <category android:name=\"android.intent.category.LAUNCHER\" />\n            </intent-filter>\n        </activity>\n"
    };
    add(
        "app/src/main/AndroidManifest.xml",
        &format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\">\n\n    <application\n        android:allowBackup=\"true\"\n        android:label=\"@string/app_name\"\n        android:supportsRtl=\"true\"\n        android:theme=\"@style/Theme.App\">\n{activity_xml}    </application>\n\n</manifest>\n"
        ),
    );
    let source =
        "app/src/main/kotlin/@PKG@/MainActivity.kt".replace("@PKG@", &package.replace('.', "/"));
    match activity {
        AndroidActivity::Compose => add(
            &source,
            r#"package @PKG@

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Text("Hello Android!")
            }
        }
    }
}
"#,
        ),
        AndroidActivity::Views => {
            add(
                &source,
                r#"package @PKG@

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
    }
}
"#,
            );
            add(
                "app/src/main/res/layout/activity_main.xml",
                "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<androidx.constraintlayout.widget.ConstraintLayout xmlns:android=\"http://schemas.android.com/apk/res/android\"\n    xmlns:app=\"http://schemas.android.com/apk/res-auto\"\n    android:layout_width=\"match_parent\"\n    android:layout_height=\"match_parent\">\n\n    <TextView\n        android:layout_width=\"wrap_content\"\n        android:layout_height=\"wrap_content\"\n        android:text=\"Hello Android!\"\n        app:layout_constraintBottom_toBottomOf=\"parent\"\n        app:layout_constraintEnd_toEndOf=\"parent\"\n        app:layout_constraintStart_toStartOf=\"parent\"\n        app:layout_constraintTop_toTopOf=\"parent\" />\n\n</androidx.constraintlayout.widget.ConstraintLayout>\n",
            );
        }
        AndroidActivity::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_id_is_a_valid_package() {
        assert_eq!(
            application_id("org.example", "My-App_2"),
            "org.example.myapp2"
        );
        assert_eq!(application_id("org.example", "---"), "org.example.app");
    }
}

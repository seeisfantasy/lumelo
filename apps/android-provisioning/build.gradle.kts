import java.nio.file.Files
import java.util.Comparator

plugins {
    id("com.android.application") version "8.13.2" apply false
}

val cleanAppleDouble by tasks.registering {
    doNotTrackState("Always clean AppleDouble sidecar files before Android builds on external volumes.")
    doLast {
        Files.walk(projectDir.toPath()).use { paths ->
            paths
                .filter { path -> path.fileName.toString().startsWith("._") }
                .sorted(Comparator.reverseOrder())
                .forEach { path -> Files.deleteIfExists(path) }
        }
    }
}

subprojects {
    tasks.matching { it.name == "preBuild" }.configureEach {
        dependsOn(rootProject.tasks.named("cleanAppleDouble"))
    }
}

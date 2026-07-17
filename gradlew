#!/bin/sh
#
# Gradle start-up script for POSIX generated for FixItGarage.
#
APP_HOME=$( cd "${0%/*}" && pwd -P ) || exit
DEFAULT_JVM_OPTS='"-Xmx64m" "-Xms64m"'
CLASSPATH=$APP_HOME/gradle/wrapper/gradle-wrapper.jar

# Determine Java
if [ -n "$JAVA_HOME" ] ; then
  JAVACMD=$JAVA_HOME/bin/java
else
  JAVACMD=java
fi

exec "$JAVACMD" $DEFAULT_JVM_OPTS -classpath "$CLASSPATH" org.gradle.wrapper.GradleWrapperMain "$@"

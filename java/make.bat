javac -d build .\webjvm\io\*.java
jar cvf ../test/java/webjvm-stdlib.jar -C ./build .

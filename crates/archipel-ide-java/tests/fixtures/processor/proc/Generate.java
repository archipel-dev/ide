package proc;

import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;

/** Consumed by an annotation processor only: the compiler keeps no trace of it in class files. */
@Retention(RetentionPolicy.SOURCE)
public @interface Generate {
}

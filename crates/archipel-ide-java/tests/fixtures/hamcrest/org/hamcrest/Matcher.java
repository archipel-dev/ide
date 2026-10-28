package org.hamcrest;

public interface Matcher<T> {
    boolean matches(Object item);
}

#include "Window.h"
#include <iostream>

Window::Window(int width, int height, const std::string& title) {
    if (!glfwInit()) exit(-1);

    glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 3);
    glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 3);
    glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);

    m_window = glfwCreateWindow(width, height, title.c_str(), NULL, NULL);
    if (!m_window) {
        glfwTerminate();
        exit(-1);
    }
    glfwMakeContextCurrent(m_window);
    gladLoadGLLoader((GLADloadproc)glfwGetProcAddress);
}

Window::~Window() {
    glfwTerminate();
}

bool Window::shouldClose() {
    return glfwWindowShouldClose(m_window);
}

bool Window::isKeyPressed(int key) {
    return glfwGetKey(m_window, key) == GLFW_PRESS;
}

void Window::update() {
    glfwSwapBuffers(m_window);
    glfwPollEvents();
}

void Window::clear(float r, float g, float b, float a) {
    glClearColor(r, g, b, a);
    glClear(GL_COLOR_BUFFER_BIT);
}


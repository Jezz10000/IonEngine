#include <iostream>
#include <vector>

#include <glad/glad.h>
#include <GLFW/glfw3.h>

#include "Window.h"
#include "Shader.h"
#include "Sprite.h"

#include "imgui.h"
#include "imgui_impl_glfw.h"
#include "imgui_impl_opengl3.h"

int main() {

    Window ionWindow(800, 600, "SCGE v0.2 Editor");

    GLFWwindow* rawWindow = ionWindow.getNativeWindow();
    
    if (!rawWindow) {
        std::cerr << "Engine Error: Failed to extract raw GLFW window context!" << std::endl;
        return -1;
    }

    Shader ourShader("C:/SCGE/shaders/default.vs", "C:/SCGE/shaders/default.fs");

    Sprite player(0.0f, 0.0f, 0.5f, 0.5f);
    Sprite enemy(0.5f, 0.5f, 0.2f, 0.2f);

    enemy.color[0] = 1.0f;
    enemy.color[1] = 0.0f;
    enemy.color[2] = 0.0f;

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO(); (void)io;
    
    ImGui::StyleColorsDark();

    ImGui_ImplGlfw_InitForOpenGL(rawWindow, true);
    ImGui_ImplOpenGL3_Init("#version 330");

    // Core Engine Loop
    while (!glfwWindowShouldClose(rawWindow)) {

        glClearColor(0.1f, 0.1f, 0.1f, 1.0f);
        glClear(GL_COLOR_BUFFER_BIT);

        ImGui_ImplOpenGL3_NewFrame();
        ImGui_ImplGlfw_NewFrame();
        ImGui::NewFrame();

        // Engine Editor

        // Panel 1
        ImGui::Begin("Inspector");
        ImGui::Text("SCGE Object Properties");
        ImGui::Separator();

        ImGui::Text("Enemy Transform:");
        ImGui::SliderFloat("Position X", &enemy.x, -1.0f, 1.0f);
        ImGui::SliderFloat("Position Y", &enemy.y, -1.0f, 1.0f);
        ImGui::SliderFloat("Scale X", &enemy.width, 0.05f, 1.0f);
        ImGui::SliderFloat("Scale Y", &enemy.height, 0.05f, 1.0f);
        
        ImGui::Separator();
        ImGui::Text("Enemy Material Color:");
        ImGui::ColorEdit3("RGB Vector", enemy.color);
        ImGui::End();

        // panel 2
        ImGui::Begin("Engine Stats");
        ImGui::Text("Live Performance: (%.1f FPS)", io.Framerate);
        ImGui::End();

        player.draw(ourShader);
        enemy.draw(ourShader);

        ImGui::Render();
        ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());

        glfwSwapBuffers(rawWindow);
        glfwPollEvents();
    }

    // UI Cleanup
    ImGui_ImplOpenGL3_Shutdown();
    ImGui_ImplGlfw_Shutdown();
    ImGui::DestroyContext();

    return 0;
}
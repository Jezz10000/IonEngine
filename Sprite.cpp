#include "Sprite.h"

Sprite::Sprite(float x, float y, float w, float h) : x(x), y(y), width(w), height(h) {
    color[0] = 1.0f; color[1] = 1.0f; color[2] = 1.0f; // Default White
    initRenderData();
}

void Sprite::initRenderData() {
    float vertices[] = {
         0.5f,  0.5f, 0.0f,
         0.5f, -0.5f, 0.0f,
        -0.5f, -0.5f, 0.0f,
        -0.5f,  0.5f, 0.0f
    };
    unsigned int indices[] = { 0, 1, 3, 1, 2, 3 };

    glGenVertexArrays(1, &VAO);
    glGenBuffers(1, &VBO);
    glGenBuffers(1, &EBO);

    glBindVertexArray(VAO);
    glBindBuffer(GL_ARRAY_BUFFER, VBO);
    glBufferData(GL_ARRAY_BUFFER, sizeof(vertices), vertices, GL_STATIC_DRAW);

    glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, EBO);
    glBufferData(GL_ELEMENT_ARRAY_BUFFER, sizeof(indices), indices, GL_STATIC_DRAW);

    glVertexAttribPointer(0, 3, GL_FLOAT, GL_FALSE, 3 * sizeof(float), (void*)0);
    glEnableVertexAttribArray(0);
}

void Sprite::draw(Shader& shader) {
    shader.use();
    glUniform2f(glGetUniformLocation(shader.ID, "offset"), x, y);
    glUniform2f(glGetUniformLocation(shader.ID, "scale"), width, height);
    glUniform4f(glGetUniformLocation(shader.ID, "ourColor"), color[0], color[1], color[2], 1.0f);
    
    glBindVertexArray(VAO);
    glDrawElements(GL_TRIANGLES, 6, GL_UNSIGNED_INT, 0);
}
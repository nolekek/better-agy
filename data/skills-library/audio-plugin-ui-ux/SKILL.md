---
name: audio-plugin-ui-ux
description: >-
  Audio plugin GUI designer and UI/UX expert. Specializes in JUCE LookAndFeel,
  vector knobs, rotary sliders, skeuomorphic analog hardware aesthetics (brushed metal, VU meters, tubes),
  modern sleek dark interfaces (FabFilter/Serum style), OpenGL GPU visualizers, and responsive HiDPI canvas.
---

# Audio Plugin UI/UX: Custom LookAndFeel & Creative Sound Interfaces

Экспертное руководство по разработке визуальных интерфейсов для аудиоплагинов: от винтажного лампового скевоморфизма до ультрасовременных векторных синтезаторов в стиле FabFilter и Serum.

---

## 🎨 Эстетические парадигмы в аудиодизайне

1. **Modern Sleek / Vector Neon (Serum, FabFilter, Vital)**:
   - Глубокий матовый темно-серый фон (`#121316`, `#1a1c23`).
   - Контрастные неоновые акценты (Cyan `#00e5ff`, Electric Amber `#ffb300`, Ultraviolet `#7c4dff`) для активных траекторий фильтров и регуляторов.
   - Минималистичные тонкие линии, полупрозрачные заливки спектров и векторная четкость на любых разрешениях (Retina / 4K).
2. **Vintage Analog / Skeuomorphic (Universal Audio, Soundtoys, Arturia)**:
   - Текстура шлифованного алюминия, винтажные крепежные винты по углам.
   - Бакелитовые ручки (knobs) с насечками, объемные тени (`DropShadowEffect`).
   - Стрелочные VU-метры с реалистичной физикой баллистики (VU-ballistics: время нарастания 300 мс).
   - Теплое янтарное свечение ламп (glow effect).

---

## 🎚️ Кастомный `juce::LookAndFeel_V4`

В JUCE все компоненты кастомизируются через переопределение методов `LookAndFeel`:

### Отрисовка профессионального поворотного регулятора (Rotary Knob)
```cpp
class AudioPluginLookAndFeel : public juce::LookAndFeel_V4 {
public:
    void drawRotarySlider(juce::Graphics& g, int x, int y, int width, int height,
                          float sliderPosProportional, float rotaryStartAngle,
                          float rotaryEndAngle, juce::Slider& slider) override
    {
        auto bounds = juce::Rectangle<int>(x, y, width, height).toFloat().reduced(6.0f);
        auto radius = juce::jmin(bounds.getWidth(), bounds.getHeight()) / 2.0f;
        auto centre = bounds.getCentre();
        auto currentAngle = rotaryStartAngle + sliderPosProportional * (rotaryEndAngle - rotaryStartAngle);

        // 1. Внешний фоновый трек (неактивная дуга)
        juce::Path backgroundArc;
        backgroundArc.addCentredArc(centre.x, centre.y, radius, radius, 0.0f,
                                    rotaryStartAngle, rotaryEndAngle, true);
        g.setColour(juce::Colour(0xff2a2d35));
        g.strokePath(backgroundArc, juce::PathStrokeType(4.0f, juce::PathStrokeType::curved, juce::PathStrokeType::rounded));

        // 2. Активный светящийся трек (значение)
        if (slider.isEnabled()) {
            juce::Path valueArc;
            valueArc.addCentredArc(centre.x, centre.y, radius, radius, 0.0f,
                                   rotaryStartAngle, currentAngle, true);
            g.setColour(juce::Colour(0xff00e5ff)); // Неоновый Cyan
            g.strokePath(valueArc, juce::PathStrokeType(4.0f, juce::PathStrokeType::curved, juce::PathStrokeType::rounded));
        }

        // 3. Тело ручки с радиальным градиентом
        auto knobBounds = bounds.reduced(8.0f);
        juce::ColourGradient knobGrad(juce::Colour(0xff22252c), centre.x, centre.y - radius * 0.5f,
                                      juce::Colour(0xff141518), centre.x, centre.y + radius * 0.5f, false);
        g.setGradientFill(knobGrad);
        g.fillEllipse(knobBounds);
        g.setColour(juce::Colour(0xff3a3f4d));
        g.drawEllipse(knobBounds, 1.5f);

        // 4. Указатель положения (Thumb pointer)
        juce::Path pointer;
        auto pointerLength = radius * 0.65f;
        pointer.startNewSubPath(centre.getPointOnCircumference(radius * 0.2f, currentAngle));
        pointer.lineTo(centre.getPointOnCircumference(pointerLength, currentAngle));
        g.setColour(juce::Colours::white);
        g.strokePath(pointer, juce::PathStrokeType(2.5f, juce::PathStrokeType::curved, juce::PathStrokeType::rounded));
    }
};
```

---

## 📊 Интерактивные визуализаторы: FFT Спектр и Кривая фильтра

Главный центр притяжения внимания в современном плагине — интерактивный дисплей:

1. **60 FPS без торможения UI**:
   - Привязка `juce::OpenGLContext` к главному компоненту плагина (`openGLContext.attachTo(*getTopLevelComponent())`). Отрисовка шейдерами снимает нагрузку с процессора.
2. **Анализатор спектра (FFT)**:
   - Преобразование Фурье выполняется в фоновом потоке или по таймеру (2048 точек).
   - Сглаживание спада спектра (Decay smoothing $\approx 0.85$), чтобы полосы плавно опускались, как в FabFilter Pro-Q.
3. **Интерактивные узлы среза (Draggable EQ Nodes)**:
   - Двойной клик на полотне создает фильтр.
   - Перетаскивание по оси X изменяет частоту среза (`Frequency` в логарифмической шкале $20 \text{ Hz} - 20 \text{ kHz}$).
   - Перетаскивание по оси Y регулирует усиление (`Gain` от $-24 \text{ dB}$ до $+24 \text{ dB}$).
   - Колесико мыши регулирует добротность (`Q factor`).

---

## 🔗 Безопасная привязка параметров (APVTS Attachments)

Никогда не меняйте параметры вручную через лямбды со слушателями. Используйте официальные RAII-аттачменты JUCE:

```cpp
// В заголовочном файле PluginEditor.h:
std::unique_ptr<juce::AudioProcessorValueTreeState::SliderAttachment> cutoffAttachment;

// В конструкторе PluginEditor.cpp:
cutoffAttachment = std::make_unique<juce::AudioProcessorValueTreeState::SliderAttachment>(
    audioProcessor.apvts, "CUTOFF_FREQ", cutoffSlider
);
// Аттачмент автоматически синхронизирует GUI, автоматизацию DAW, отмену Undo/Redo и хост
```

---

## 📐 Масштабируемость и HiDPI (Retina / 4K)

- Плагин обязан поддерживать изменение размера окна пользователем (`setResizable(true, true)`).
- Используйте `juce::ComponentBoundsConstrainer` с фиксированным соотношением сторон (например, 16:10 или 4:3).
- Вся графика должна быть векторной (`juce::Path`, `.reduced()`, `.toFloat()`). Никаких растровых картинок с фиксированными пикселями!

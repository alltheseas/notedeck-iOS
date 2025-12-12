//
//  NotedeckUIView.swift
//  NotedeckMobile
//

import UIKit
import QuartzCore
import NotedeckMobile

class NotedeckUIView: UIView {
    override class var layerClass: AnyClass {
        CAMetalLayer.self
    }

    var metalLayer: CAMetalLayer {
        layer as! CAMetalLayer
    }

    private var gatheredEvents: [InputEvent] = []
    private var activeTouches: [UITouch: CGPoint] = [:]

    // Native text field overlay for keyboard input
    private let nativeTextField = NativeTextFieldOverlay()
    private var lastWantsKeyboard = false
    private var lastImeRect: CGRect?
    private var isShowingNativeTextField = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        setupNativeTextField()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setupNativeTextField()
    }

    private func setupNativeTextField() {
        addSubview(nativeTextField)

        nativeTextField.onTextChanged = { [weak self] oldText, newText in
            guard let self = self else { return }

            if newText.count > oldText.count && newText.hasPrefix(oldText) {
                let addedText = String(newText.dropFirst(oldText.count))
                self.gatheredEvents.append(.from_text_commit(addedText))
            } else if newText.count < oldText.count && oldText.hasPrefix(newText) {
                let deletedCount = oldText.count - newText.count
                for _ in 0..<deletedCount {
                    self.gatheredEvents.append(.from_virtual_key(0, true))
                    self.gatheredEvents.append(.from_virtual_key(0, false))
                }
            } else if newText != oldText {
                for _ in 0..<oldText.count {
                    self.gatheredEvents.append(.from_virtual_key(0, true))
                    self.gatheredEvents.append(.from_virtual_key(0, false))
                }
                if !newText.isEmpty {
                    self.gatheredEvents.append(.from_text_commit(newText))
                }
            }
        }

        nativeTextField.onReturnKey = { [weak self] in
            guard let self = self else { return }
            self.gatheredEvents.append(.from_virtual_key(1, true))
            self.gatheredEvents.append(.from_virtual_key(1, false))
        }

        nativeTextField.onDismiss = { [weak self] in
            self?.isShowingNativeTextField = false
        }

        nativeTextField.onCopy = { [weak self] in
            self?.gatheredEvents.append(.from_copy())
        }

        nativeTextField.onCut = { [weak self] in
            self?.gatheredEvents.append(.from_cut())
        }

        nativeTextField.onPaste = { [weak self] text in
            self?.gatheredEvents.append(.from_paste(text))
        }
    }

    func drainEvents() -> RustVec<InputEvent> {
        let events = gatheredEvents
        gatheredEvents = []

        let result = RustVec<InputEvent>()
        for event in events {
            result.push(value: event)
        }
        return result
    }

    func handle(output: OutputState) {
        let wantsKeyboard = output.wants_keyboard()

        var imeRect: CGRect? = nil
        if output.has_ime_rect() {
            imeRect = CGRect(
                x: CGFloat(output.get_ime_rect_x()),
                y: CGFloat(output.get_ime_rect_y()),
                width: CGFloat(output.get_ime_rect_width()),
                height: CGFloat(output.get_ime_rect_height())
            )
        }

        if wantsKeyboard && !isShowingNativeTextField {
            if let rect = imeRect {
                nativeTextField.show(at: rect, in: self)
            } else {
                nativeTextField.showAtDefaultPosition(in: self)
            }
            isShowingNativeTextField = true
        } else if !wantsKeyboard && isShowingNativeTextField {
            nativeTextField.hide()
            isShowingNativeTextField = false
        } else if wantsKeyboard && isShowingNativeTextField, let rect = imeRect {
            if lastImeRect != rect {
                nativeTextField.frame = CGRect(
                    x: rect.origin.x,
                    y: rect.origin.y - 2,
                    width: rect.width,
                    height: max(rect.height + 4, 36)
                )
            }
        }

        lastWantsKeyboard = wantsKeyboard
        lastImeRect = imeRect

        let copiedText = output.get_copied_text().toString()
        if !copiedText.isEmpty {
            UIPasteboard.general.string = copiedText
        }
    }

    // MARK: - Touch Handling

    override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
        if !nativeTextField.isHidden && nativeTextField.frame.contains(point) {
            return nativeTextField.hitTest(convert(point, to: nativeTextField), with: event)
        }
        return super.hitTest(point, with: event)
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesBegan(touches, with: event)

        for touch in touches {
            let point = touch.location(in: self)

            if isShowingNativeTextField && !nativeTextField.frame.contains(point) {
                nativeTextField.hide()
                isShowingNativeTextField = false
            }

            activeTouches[touch] = point
            gatheredEvents.append(
                .from_pointer_moved(Float(point.x), Float(point.y))
            )
            gatheredEvents.append(
                .from_left_mouse_down(Float(point.x), Float(point.y), true)
            )
        }
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesMoved(touches, with: event)
        for touch in touches {
            let point = touch.location(in: self)
            activeTouches[touch] = point
            gatheredEvents.append(
                .from_pointer_moved(Float(point.x), Float(point.y))
            )
        }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesEnded(touches, with: event)
        for touch in touches {
            let point = touch.location(in: self)
            activeTouches.removeValue(forKey: touch)
            gatheredEvents.append(
                .from_left_mouse_down(Float(point.x), Float(point.y), false)
            )
        }
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesCancelled(touches, with: event)
        for touch in touches {
            let point = touch.location(in: self)
            activeTouches.removeValue(forKey: touch)
            gatheredEvents.append(
                .from_left_mouse_down(Float(point.x), Float(point.y), false)
            )
        }
    }
}

// MARK: - Native Text Field Overlay

class NativeTextFieldOverlay: UITextField, UITextFieldDelegate {

    var onTextChanged: ((String, String) -> Void)?
    var onReturnKey: (() -> Void)?
    var onDismiss: (() -> Void)?
    var onCopy: (() -> Void)?
    var onCut: (() -> Void)?
    var onPaste: ((String) -> Void)?

    private var previousText: String = ""

    override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    private func setup() {
        delegate = self

        backgroundColor = UIColor.systemBackground
        textColor = UIColor.label
        font = UIFont.systemFont(ofSize: 16)
        borderStyle = .roundedRect
        autocorrectionType = .yes
        autocapitalizationType = .sentences
        returnKeyType = .send
        clearButtonMode = .whileEditing

        leftView = UIView(frame: CGRect(x: 0, y: 0, width: 8, height: 0))
        leftViewMode = .always

        isHidden = true
        alpha = 0

        addTarget(self, action: #selector(textDidChange), for: .editingChanged)
    }

    @objc private func textDidChange() {
        let newText = text ?? ""
        onTextChanged?(previousText, newText)
        previousText = newText
    }

    func textFieldShouldReturn(_ textField: UITextField) -> Bool {
        onReturnKey?()
        return false
    }

    override func copy(_ sender: Any?) {
        super.copy(sender)
        onCopy?()
    }

    override func cut(_ sender: Any?) {
        super.cut(sender)
        onCut?()
    }

    override func paste(_ sender: Any?) {
        let clipboardText = UIPasteboard.general.string ?? ""
        super.paste(sender)
        onPaste?(clipboardText)
    }

    func show(at rect: CGRect, in parentView: UIView) {
        frame = CGRect(
            x: rect.origin.x,
            y: rect.origin.y - 2,
            width: rect.width,
            height: max(rect.height + 4, 36)
        )

        isHidden = false

        UIView.animate(withDuration: 0.2) {
            self.alpha = 1
        }

        becomeFirstResponder()
    }

    func showAtDefaultPosition(in parentView: UIView) {
        let safeArea = parentView.safeAreaInsets
        let width = parentView.bounds.width - 32
        let height: CGFloat = 44
        let y = parentView.bounds.height - safeArea.bottom - height - 16

        frame = CGRect(x: 16, y: y, width: width, height: height)

        isHidden = false

        UIView.animate(withDuration: 0.2) {
            self.alpha = 1
        }

        becomeFirstResponder()
    }

    func hide() {
        resignFirstResponder()

        UIView.animate(withDuration: 0.15) {
            self.alpha = 0
        } completion: { _ in
            self.isHidden = true
            self.text = ""
            self.previousText = ""
        }

        onDismiss?()
    }
}

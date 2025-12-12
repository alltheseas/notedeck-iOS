//
//  NotedeckRendererController.swift
//  NotedeckMobile
//

import Metal
import QuartzCore
import UIKit
import NotedeckMobile

@MainActor
final class NotedeckRendererController: NSObject {
    private weak var view: NotedeckUIView!
    private var displayLink: CADisplayLink!
    private var renderer: NotedeckRenderer!

    override init() {
        super.init()
    }

    deinit {
        MainActor.assumeIsolated {
            displayLink?.invalidate()
        }
    }

    func initialize(view: NotedeckUIView, size: CGSize, scale: CGFloat) {
        self.view = view

        let layer = view.metalLayer
        layer.framebufferOnly = true
        layer.pixelFormat = .bgra8Unorm_srgb
        layer.drawableSize = CGSize(width: size.width * scale, height: size.height * scale)
        layer.contentsScale = scale

        // Pass the UIView pointer - wgpu extracts CAMetalLayer from it
        let rawPointer = Unmanaged.passUnretained(view).toOpaque()

        // Get the app's data directory
        let dataPath = getDataPath()

        renderer = NotedeckRenderer(
            rawPointer,
            UInt32(max(size.width, 100.0) * scale),
            UInt32(max(size.height, 100.0) * scale),
            Float(scale),
            dataPath
        )

        // Set initial safe area
        updateSafeArea()
    }

    func updateSafeArea() {
        guard let window = view.window else { return }
        let safeArea = window.safeAreaInsets
        renderer.set_safe_area(
            Float(safeArea.top),
            Float(safeArea.right),
            Float(safeArea.bottom),
            Float(safeArea.left)
        )
    }

    func start(view: NotedeckUIView) {
        let link = CADisplayLink(target: self, selector: #selector(renderFrame))
        link.add(to: .main, forMode: .common)
        self.displayLink = link
    }

    private var lastSafeAreaTop: CGFloat = 0

    @objc
    func renderFrame() {
        // Update safe area if window became available or safe area changed
        if let window = view.window {
            let safeArea = window.safeAreaInsets
            if safeArea.top != lastSafeAreaTop {
                lastSafeAreaTop = safeArea.top
                updateSafeArea()
            }
        }

        let events = view.drainEvents()
        let timestamp = CACurrentMediaTime()
        let state = renderer.render(timestamp, events)
        view.handle(output: state)
    }

    func resize(to size: CGSize, scale: CGFloat) {
        guard size.width > 0 && size.height > 0 else {
            return
        }
        view.metalLayer.drawableSize = CGSize(width: size.width * scale, height: size.height * scale)
        renderer.resize(
            UInt32(size.width * scale),
            UInt32(size.height * scale)
        )
    }

    private func getDataPath() -> String {
        let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
        return paths[0].path
    }
}

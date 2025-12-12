//
//  NotedeckView.swift
//  NotedeckMobile
//

import SwiftUI
import UIKit
import NotedeckMobile

struct NotedeckView: View {
    @Environment(\.displayScale)
    var displayScale

    var body: some View {
        GeometryReader { proxy in
            NotedeckUIViewRepresentable(
                size: proxy.size,
                scale: displayScale
            )
            .ignoresSafeArea()
        }
    }
}

private struct NotedeckUIViewRepresentable: UIViewRepresentable {
    let size: CGSize
    let scale: CGFloat

    func makeUIView(context: Context) -> NotedeckUIView {
        let view = NotedeckUIView(frame: .zero)
        context.coordinator.initialize(view: view, size: size, scale: scale)
        context.coordinator.start(view: view)
        return view
    }

    func updateUIView(_ uiView: NotedeckUIView, context: Context) {
        context.coordinator.resize(to: size, scale: scale)
    }

    func makeCoordinator() -> NotedeckRendererController {
        NotedeckRendererController()
    }
}

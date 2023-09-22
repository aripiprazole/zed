use crate::{
    AnyElement, Element, Handle, IntoAnyElement, Layout, LayoutId, Result, ViewContext,
    WindowContext,
};
use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

pub struct View<S, P> {
    state: Handle<S>,
    render: Rc<dyn Fn(&mut S, &mut ViewContext<S>) -> AnyElement<S>>,
    parent_state_type: PhantomData<P>,
}

impl<S, P> Clone for View<S, P> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            render: self.render.clone(),
            parent_state_type: PhantomData,
        }
    }
}

pub type RootView<S> = View<S, ()>;

pub fn view<S: 'static, P: 'static, E: Element<State = S>>(
    state: Handle<S>,
    render: impl 'static + Fn(&mut S, &mut ViewContext<S>) -> E,
) -> View<S, P> {
    View {
        state,
        render: Rc::new(move |state, cx| render(state, cx).into_any()),
        parent_state_type: PhantomData,
    }
}

impl<S: 'static, P: 'static> View<S, P> {
    pub fn into_any<ParentState>(self) -> AnyView<ParentState> {
        AnyView {
            view: Rc::new(RefCell::new(self)),
            parent_state_type: PhantomData,
        }
    }
}

impl<S: 'static, P: 'static> Element for View<S, P> {
    type State = P;
    type FrameState = AnyElement<S>;

    fn layout(
        &mut self,
        _: &mut Self::State,
        cx: &mut ViewContext<Self::State>,
    ) -> Result<(LayoutId, Self::FrameState)> {
        self.state.update(cx, |state, cx| {
            let mut element = (self.render)(state, cx);
            let layout_id = element.layout(state, cx)?;
            Ok((layout_id, element))
        })
    }

    fn paint(
        &mut self,
        layout: Layout,
        _: &mut Self::State,
        element: &mut Self::FrameState,
        cx: &mut ViewContext<Self::State>,
    ) -> Result<()> {
        self.state
            .update(cx, |state, cx| element.paint(state, None, cx))
    }
}

trait ViewObject {
    fn layout(&mut self, cx: &mut WindowContext) -> Result<(LayoutId, Box<dyn Any>)>;
    fn paint(
        &mut self,
        layout: Layout,
        element: &mut dyn Any,
        cx: &mut WindowContext,
    ) -> Result<()>;
}

impl<S: 'static, P> ViewObject for View<S, P> {
    fn layout(&mut self, cx: &mut WindowContext) -> Result<(LayoutId, Box<dyn Any>)> {
        self.state.update(cx, |state, cx| {
            let mut element = (self.render)(state, cx);
            let layout_id = element.layout(state, cx)?;
            let element = Box::new(element) as Box<dyn Any>;
            Ok((layout_id, element))
        })
    }

    fn paint(
        &mut self,
        layout: Layout,
        element: &mut dyn Any,
        cx: &mut WindowContext,
    ) -> Result<()> {
        self.state.update(cx, |state, cx| {
            element
                .downcast_mut::<AnyElement<S>>()
                .unwrap()
                .paint(state, None, cx)
        })
    }
}

pub struct AnyView<S> {
    view: Rc<RefCell<dyn ViewObject>>,
    parent_state_type: PhantomData<S>,
}

impl<S: 'static> Element for AnyView<S> {
    type State = S;
    type FrameState = Box<dyn Any>;

    fn layout(
        &mut self,
        _: &mut Self::State,
        cx: &mut ViewContext<Self::State>,
    ) -> Result<(LayoutId, Self::FrameState)> {
        self.view.borrow_mut().layout(cx)
    }

    fn paint(
        &mut self,
        layout: Layout,
        _: &mut Self::State,
        element: &mut Self::FrameState,
        cx: &mut ViewContext<Self::State>,
    ) -> Result<()> {
        self.view.borrow_mut().paint(layout, element, cx)
    }
}

impl<S> Clone for AnyView<S> {
    fn clone(&self) -> Self {
        Self {
            view: self.view.clone(),
            parent_state_type: PhantomData,
        }
    }
}

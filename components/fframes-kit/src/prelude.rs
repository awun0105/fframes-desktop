//! Everything a video needs: `use fframes_kit::prelude::*;`.

pub use fframes::{
    AudioMap, Color, Duration, FFramesContext, FontQuery, Frame, Overlap, Scene, Scenes, Svgr,
    TextAlign, Transform, Video, animation::Easing, svgr, timeline,
};

pub use crate::anim::{
    Anim, ComplexValueTracker, Enter, Exit, FadeIn, FadeOut, RateFunc, ValueTracker,
};
pub use crate::animate::{
    AnimationGroup, Circumscribe, Create, DrawBorderThenFill, Emphasis, EnterAnim, ExitAnim, Flash,
    GrowArrow, GrowFromCenter, GrowFromPoint, Indicate, LaggedStart, MoveAlongPath,
    ReplacementTransform, SpinInFromNothing, Succession, Transform as Morph, Uncreate, Wiggle,
    Write,
};
pub use crate::audio::{self, BarAlign, SpectrumBars, Waveform};
pub use crate::data::BarChart;
pub use crate::decor::{CornerMarks, DrawOn, DropShadow, Glow, PhotoFrame};
#[cfg(feature = "skia")]
pub use crate::effects::ShaderNoise;
pub use crate::effects::{
    BevelAlpha, BlindsDirection, Blur, Brightness, Burlap, CameraMotionBlur, Checkerboard,
    ColorCorrection, ColorKey, ColorMatrix, ColorStack, Contrast, Curves, DotGrid, Duotone, Emboss,
    Evolve, EvolveDirection, Exposure, Flannel, FractalNoise, FractalType, Grayscale, Gridlines,
    Halftone, HalftoneColor, HalftoneLinearGradient, HalftoneShape, HueRotate, Invert, Levels,
    LightLeak, LightTrail, LineDirection, LinearGradient, LinearGradientTint,
    LinearProgressiveBlur, Lines, LongShadows, Lut, Mirror, MirrorDirection, Noise, Outline, Paper,
    Pattern, RadialProgressiveBlur, RegionBlur, Rings, RoughenEdges, Saturation, Scale, Scanlines,
    ShadowsHighlights, Shrinkwrap, Skew, Speckle, Starburst, ThermalVision, Tile, Tint, Trail,
    Translate, TvSignalOff, VenetianBlinds, Vibrance, Vignette, VignetteMode, WhiteBalance,
    WhiteNoise,
};
pub use crate::explain::{
    Area, ArrowVectorField, Axes, AxesMap, AzimuthDirection, AzimuthUnits, Canvas, ComplexPlane,
    Curve, Flow, Func, FunctionGraph, Implicit, ImplicitFunction, Matrix, NumberLine, NumberPlane,
    ParametricFunction, PolarPlane, Range, RiemannRectangles, SampleAt, SampleColumn, SampleRow,
    SampleSpace, Scaling, Space, StreamLines, Table, VGroup, VectorField,
};
pub use crate::geometry::{Geometry, Look, Shape};
#[cfg(all(feature = "biology", feature = "3d"))]
pub use crate::kits::biology::DnaHelix3D;
#[cfg(feature = "biology")]
pub use crate::kits::biology::{
    ActionPotential, BaseColors, Cell, CellKind, CellPalette, Circulation, DnaHelix, EnzymeFit,
    FoodWeb, MitosisStage, MitosisStages, Neuron, Organelle, PhyloTree, PopulationCurve,
    PopulationModel, PunnettSquare, Transcription, Translation,
};
#[cfg(all(feature = "chemistry", feature = "latex"))]
pub use crate::kits::chemistry::ReactionEquation;
#[cfg(all(feature = "chemistry", feature = "3d"))]
pub use crate::kits::chemistry::{Atom3D, Molecule3D, Orbital, OrbitalKind, Structure3D, XyzError};
#[cfg(feature = "chemistry")]
pub use crate::kits::chemistry::{
    BohrAtom, BondFormation, BondKind, BondType, Category, ChemAtom, ChemBond, ElementTile,
    EnergyProfile, Equilibrium, EquilibriumModel, MatterState, Molecule, Molecule2D, PeriodicTable,
    PhScale, Reaction, SmilesError, StatesOfMatter, Structure, Titration, TitrationCurve, Trend,
};
#[cfg(feature = "cs")]
pub use crate::kits::cs::{
    ArrayStep, ArrayTrace, ArrayView, BinarySearch, CallStack, CallTrace, CellColors, CodeBlock,
    ComplexityPlot, Graph, GraphTrace, GraphView, Growth, HashTable, HashTrace, Language,
    LinkedList, ListTrace, Machine, MachineTrace, Message, Queue, RecursionTree, SequenceDiagram,
    SortVisualizer, Stack, StateMachine, TreeTrace, TreeView,
};
#[cfg(feature = "math")]
pub use crate::kits::math::{
    AreaUnder, Epicycles, FractionBar, FractionPie, FunctionPlot, Interval, LinearTransform,
    MatrixView, NormalFit, NumberPoint, Pythagoras, RiemannSum, Secant, Tangent, UnitCircle,
    VectorSum,
};
#[cfg(feature = "physics")]
pub use crate::kits::physics::{
    Charge, Circuit, Collision1D, Doppler, EnergyBars, Field, FieldLines, Force, FreeBody, GasBox,
    GasMotion, GasSetup, Heat, Optic, Orbit, Pendulum, PendulumMotion, Projectile, RayDiagram,
    Refraction, SpringMass, SpringMotion, Wave, WaveKind,
};
pub use crate::label::{BraceLabel, Label, Leader};
#[cfg(feature = "latex")]
pub use crate::latex::{
    ChemEq, Formula, FormulaView, MathTex, Part, Tex, TexEngine, TexError, TexTemplate,
    TransformMatchingTex,
};
pub use crate::layout::Anchor;
pub use crate::media::KenBurns;
pub use crate::motion::{self, Beat};
pub use crate::overlay::{
    Annotation, Appear, Caption, CaptionStack, Captions, ChapterList, Check, LowerThird,
    ProgressBar,
};
pub use crate::paths;
pub use crate::plot::{Heatmap, Histogram, LinePlot, ScatterPlot};
pub use crate::props::{Editable, EditableExt};
#[cfg(feature = "studio")]
pub use crate::props::{PropChoice, PropValue, Props, PropsStore};
pub use crate::shapes::{
    Angle, AnnotationDot, AnnularSector, Annulus, Arc, ArcBetweenPoints, ArcBrace, ArcPolygon,
    ArcPolygonFromArcs, Arrow, ArrowTip, BackgroundRectangle, BlockArrow, BooleanOp, Brace,
    Callout, Circle, ConvexHull, Cross, CubicBezier, CurvedArrow, CurvedDoubleArrow,
    CurvesAsSubmobjects, Cutout, DashedLine, DashedVMobject, Difference, Dot, DoubleArrow, Elbow,
    Ellipse, Exclusion, FullScreenRectangle, Heart, Intersection, LabeledArrow, LabeledDot,
    LabeledLine, LabeledPolygram, Line, Pie, Polygon, Polygram, Rect, Rectangle, RegularPolygon,
    RegularPolygram, RightAngle, RoundedRectangle, SMALL_BUFF, ScreenRectangle, Sector, ShapeLabel,
    Spark, Square, Star, SurroundingRectangle, SvgPath, SvgShape, TangentLine, TangentialArc,
    TextPath, Triangle, Underline, Union, Vector,
};
pub use crate::sim::{Lerp, Sim};
pub use crate::text::{
    BulletedList, Counter, DecimalNumber, FitText, HudLabel, Integer, LetterLook, Letters,
    MarkupText, Paragraph, RevealBy, TextAnchor, TextReveal, Title, Typewriter, Variable,
};
pub use crate::theme::{
    Colors, Durations, Easings, Motion, Radius, Shadow, Shadows, Spacing, Stagger, Stroke,
    TextStyle, Theme, Typography,
};
pub use crate::time::{Fill, Freeze, Loop, Sequence, Series};
pub use crate::transitions::{
    AudioFade, BlurSlide, CircleWipe, ClockWipe, CrossZoom, Direction, Dissolve, DreamyZoom, Fade,
    FilmBurn, Flip, Iris, LinearBlur, NoTransition, Presentation, PushCut, SceneFade, SeriesLength,
    Side, SlantWipe, Slide, SwapCards, Timing, TransitionItem, TransitionProgress,
    TransitionSeries, Wipe, ZoomInOut, audio_fades, scene_transition,
};
pub use crate::ui::{BoxedComponent, Children, Component, DynComponent, Themed, Ui, children};
pub use fframes_kit_macros::ui;
pub use kurbo::{BezPath, Point};

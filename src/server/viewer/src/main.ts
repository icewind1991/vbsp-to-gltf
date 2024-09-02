import './style.css'
import * as THREE from 'three'
import {PointerLockControls} from 'three/addons/controls/PointerLockControls.js'
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js'
import Stats from 'three/addons/libs/stats.module.js'
import {KTX2Loader} from 'three/addons/loaders/KTX2Loader.js';

const scene = new THREE.Scene()

const camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 10000)
camera.position.set(0, 0, 0)

const renderer = new THREE.WebGLRenderer({antialias: true})


const loader = new GLTFLoader();

const ktx2Loader = new KTX2Loader();
ktx2Loader.setTranscoderPath('/transcoders/');
ktx2Loader.detectSupport(renderer);

loader.setKTX2Loader(ktx2Loader);

renderer.setSize(window.innerWidth, window.innerHeight)
document.body.appendChild(renderer.domElement)

window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight
    camera.updateProjectionMatrix()
    renderer.setSize(window.innerWidth, window.innerHeight)
})

const startButton = document.getElementById('startButton') as HTMLButtonElement
startButton.addEventListener(
    'click',
    () => {
        controls.lock()
    },
    false
)

const controls = new PointerLockControls(camera, renderer.domElement)
controls.pointerSpeed = 2;
controls.addEventListener('lock', () => (startButton.style.display = 'none'))
controls.addEventListener('unlock', () => (startButton.style.display = 'block'))

// const controls = new OrbitControls(camera, renderer.domElement)
// controls.enableDamping = true


const ambientLight = new THREE.AmbientLight(0xffffff, 0.4);
scene.add(ambientLight);

const dirLight = new THREE.DirectionalLight(0xefefff, 1.5);
dirLight.position.set(10, 10, 10);
scene.add(dirLight);

let map;
let base_url = "";

const urlParams = new URLSearchParams(window.location.search);
if (window.location.pathname.startsWith('/view/')) {
    map = window.location.pathname.substring('/view/'.length);
} else {
    map = urlParams.get('map');
    base_url = "https://gltf.demos.tf"
}
const textureScale = urlParams.get('texture_scale') || 0.25;
const textures = urlParams.get('textures') || true;
console.log(map);

loader.load(`${base_url}/gltf/${map}.glb?texture_scale=${textureScale}&textures=${textures}`, (gltf) => {
    document.body.classList.remove('loading');
    gltf.scene.traverse(child => {
        if ((child as THREE.Mesh).material) {
            ((child as THREE.Mesh).material as any as THREE.MaterialJSON).metalness = 0;
        }
    });
    scene.add(gltf.scene)
}, () => {
}, (e) => {
    (document.getElementById('loading') as HTMLElement).textContent = `Failed to load map: ${e}`;
})

const stats = new Stats()
document.body.appendChild(stats.dom)

const clock = new THREE.Clock()
let delta

const keyMap: { [key: string]: boolean } = {}
const onDocumentKey = (e: KeyboardEvent) => {
    keyMap[e.code] = e.type === 'keydown'
}
document.addEventListener('keydown', onDocumentKey, false)
document.addEventListener('keyup', onDocumentKey, false)

let movementScale = 250;


const _vector = new THREE.Vector3();

function moveForward(distance: number): void {
    _vector.copy(controls.getDirection(_vector));

    camera.position.addScaledVector(_vector, distance);
}

function animate() {
    requestAnimationFrame(animate)

    delta = clock.getDelta()

    let moving = false;

    if (keyMap['KeyW'] || keyMap['ArrowUp']) {
        moving = true;
        moveForward(delta * movementScale)
    }
    if (keyMap['KeyS'] || keyMap['ArrowDown']) {
        moving = true;
        moveForward(-delta * movementScale)
    }
    if (keyMap['KeyA'] || keyMap['ArrowLeft']) {
        moving = true;
        controls.moveRight(-delta * movementScale)
    }
    if (keyMap['KeyD'] || keyMap['ArrowRight']) {
        moving = true;
        controls.moveRight(delta * movementScale)
    }

    if (moving) {
        movementScale = movementScale * (1 + 1.5 * delta);
        movementScale = Math.min(movementScale, 1500);
    } else {
        movementScale = 250;
    }

    renderer.render(scene, camera)

    stats.update()
}

animate()